//! The turn engine: a synchronous sans-io state machine (SPEC section 6).
//!
//! Drivers feed [`CoreInput`]s in and discharge the bounded [`CoreAction`]
//! batches that come back. The engine owns session/turn state, the
//! exactly-once terminal ledger (first terminal wins; late or duplicate
//! completions are dropped with a trace record, SPEC L-T1), epoch-scoped
//! cancellation (L-T2), the stationarity guard (L-T3), and window assembly.
//! `handle` is never re-entrant and never blocks: verbs and tools that need
//! data or time leave as effects (L-A1).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::error::{ErrorCode, ErrorEvent, VerbErrorCause};
use kittens_code_protocol::event::{Event, ToolOutcome, TurnEnd};
use kittens_code_protocol::ids::{EffectId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use kittens_code_protocol::policy::{ApprovalPolicy, ApprovalVerdict};

use crate::caps::{Capped, ToolResult as ToolResultCap};
use crate::compact::CompactionState;
use crate::prompts::{self, TemplateId};
use crate::record::{Record, RecordKind, RecordPayload};
use crate::rlm::exec::{AskRequest, AskResult, Bound, Executor, Page, StepOutcome};
use crate::rlm::ir::{BoundValue, Sel};
use crate::rlm::lower::lower_script;
use crate::tokens::TokenAccounting;
use crate::window::{TailItem, WindowLayout};

/// Maximum tool calls accepted from one model response. A response
/// proposing more is truncated to this many with a trace record, so the
/// resulting transition stays bounded (review input 19 #9: the previous
/// `MAX_ACTIONS` debug-assert was not a real bound). Each accepted call
/// contributes a small constant number of actions, so the transition size
/// is `O(MAX_TOOL_CALLS_PER_TURN)`.
pub const MAX_TOOL_CALLS_PER_TURN: usize = 32;

/// Bytes reserved within the tool-result budget for the truncation
/// annotation, so the surfaced value never exceeds the declared cap.
const ANNOTATION_RESERVE: u32 = 96;

/// A tool call proposed by the model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposedToolCall {
    /// Tool name.
    pub name: String,
    /// JSON-encoded arguments as the model produced them.
    pub args_json: String,
}

/// Provider usage attached to a model terminal (C8 calibration input).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Usage {
    /// Prompt tokens the provider reported.
    pub prompt_tokens: u64,
    /// Bytes our window measured for the same content.
    pub prompt_bytes: u64,
}

/// How a model call finished.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelOutcome {
    /// Final assistant text (may be empty when only tools were called).
    pub text: String,
    /// Proposed tool calls, in model order.
    pub tool_calls: Vec<ProposedToolCall>,
    /// Provider usage, when reported.
    pub usage: Option<Usage>,
}

/// Terminal payload of a finished effect.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EffectTerminal {
    /// A root model call finished.
    Model(ModelOutcome),
    /// A tool finished; `output` is the full untruncated output.
    Tool {
        /// Outcome class.
        outcome: ToolOutcome,
        /// Full output (the engine truncates for the window; the full
        /// value is committed to the log — reversible offload, Q3).
        output: String,
    },
    /// A transcript store-page read finished.
    Pages(Page),
    /// One or more sub-model requests finished.
    Ask(Vec<AskResult>),
    /// The effect failed at the driver layer.
    Failed {
        /// The failure.
        error: ErrorCode,
        /// Human-readable context.
        message: String,
    },
}

/// What a started effect asks the driver to do.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EffectSpec {
    /// Call the root model with this assembled window.
    ModelCall(WindowLayout),
    /// Execute a tool.
    Tool(ProposedToolCall),
    /// Read one rendered page from the transcript store.
    StoreReadPage {
        /// The transcript selection to page through.
        sel: Sel,
        /// Driver-defined continuation cursor, or `None` for the first page.
        cursor: Option<u64>,
    },
    /// Run one or more sub-model asks.
    ///
    /// KC0 starts singleton request vectors so each answer has its own
    /// exactly-once child effect; the vector keeps the seam additive for
    /// future batched drivers.
    SubModel {
        /// Requests to resolve into matching [`AskResult`] values.
        requests: Vec<AskRequest>,
    },
}

/// One input into the engine.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CoreInput {
    /// A client submission.
    ClientOp(Submission),
    /// An effect finished (exactly once per effect; enforced here anyway).
    EffectFinished {
        /// The effect.
        id: EffectId,
        /// The epoch it was started under.
        epoch: TurnEpoch,
        /// Its terminal payload.
        terminal: EffectTerminal,
    },
    /// The appender's durability watermark advanced.
    Persisted {
        /// All records with `seq <= up_to_seq` are durable.
        up_to_seq: u64,
    },
    /// The appender failed; the session cannot keep acting (append canon).
    PersistFailed {
        /// The failing sequence.
        at_seq: u64,
        /// Driver-supplied context.
        message: String,
    },
}

/// One action for the driver to discharge, in order (L-A2b).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CoreAction {
    /// Append these records through the log-appender.
    Commit(Vec<Record>),
    /// Publish this event (driver holds it until the covering watermark;
    /// the record carrying it is in the same transition's Commit).
    Publish(Event),
    /// Start an effect.
    StartEffect {
        /// The new effect's identity.
        id: EffectId,
        /// The epoch it belongs to.
        epoch: TurnEpoch,
        /// What to do.
        spec: EffectSpec,
    },
    /// Cancel an in-flight effect.
    CancelEffect {
        /// The effect to cancel.
        id: EffectId,
    },
}

/// A bounded, owned action batch (never a lazy iterator; L-A2).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Transition {
    /// Actions in dispatch order.
    pub actions: Vec<CoreAction>,
}

/// Where the session currently stands.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Phase {
    /// Between turns.
    Idle,
    /// A root model call is in flight.
    AwaitingModel(EffectId),
    /// Tool calls are running; the turn resamples when all are terminal.
    RunningTools,
    /// Shutdown requested; no new effects start.
    Draining,
    /// The appender failed; only draining remains.
    Failed,
}

/// Why a persisted transcript could not seed a resumed engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResumeError {
    /// The replay slice was empty or its first record was not a valid header.
    MissingHeader,
    /// The log already used the largest sequence number.
    SequenceExhausted,
    /// The log already used the largest effect id.
    EffectIdExhausted,
    /// The log already used the largest request/submission id.
    SubmissionIdExhausted,
    /// The log already used the largest turn epoch.
    TurnEpochExhausted,
}

impl fmt::Display for ResumeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingHeader => "replay must start with a valid header record",
            Self::SequenceExhausted => "record sequence namespace is exhausted",
            Self::EffectIdExhausted => "effect id namespace is exhausted",
            Self::SubmissionIdExhausted => "request/submission id namespace is exhausted",
            Self::TurnEpochExhausted => "turn epoch namespace is exhausted",
        };
        formatter.write_str(message)
    }
}

impl core::error::Error for ResumeError {}

#[derive(Default)]
struct ReplayMaxima {
    seq: u64,
    effect: u64,
    submission: u64,
    epoch: u64,
}

impl ReplayMaxima {
    fn observe_record(&mut self, record: &Record) {
        self.seq = self.seq.max(record.seq);
        self.epoch = self.epoch.max(record.epoch.0);
        if let Some(id) = record.txn {
            self.observe_effect(id);
        }
        match &record.payload {
            RecordPayload::AcceptedOp(submission) => self.observe_submission(submission),
            RecordPayload::EmittedEvent(event) => self.observe_event(event),
            _ => {}
        }
    }

    fn observe_submission(&mut self, submission: &Submission) {
        self.observe_request(submission.id);
        if let Op::Approve { request, .. } = &submission.op {
            self.observe_request(*request);
        }
    }

    fn observe_event(&mut self, event: &Event) {
        match event {
            Event::TurnStarted { epoch, correlates } => {
                self.epoch = self.epoch.max(epoch.0);
                if let Some(id) = correlates {
                    self.observe_request(*id);
                }
            }
            Event::TurnEnded { epoch, .. }
            | Event::ModelDelta { epoch, .. }
            | Event::CompactionStarted { epoch }
            | Event::CompactionApplied { epoch }
            | Event::CompactionSuppressed { epoch } => {
                self.epoch = self.epoch.max(epoch.0);
            }
            Event::ToolProposed { call, .. }
            | Event::ToolStarted { call }
            | Event::ToolOutputDelta { call, .. }
            | Event::ToolTerminal { call, .. } => self.observe_effect(*call),
            Event::ApprovalRequested { request, call, .. } => {
                self.observe_request(*request);
                self.observe_effect(*call);
            }
            Event::QueryTrace { query, .. } => self.observe_effect(*query),
            Event::Error(error) => {
                if let Some(id) = error.correlates {
                    self.observe_request(id);
                }
            }
            _ => {}
        }
    }

    fn observe_effect(&mut self, id: EffectId) {
        self.effect = self.effect.max(id.0);
    }

    fn observe_request(&mut self, id: SubmissionId) {
        self.submission = self.submission.max(id.0);
    }
}

/// One in-flight tool call slot (rejoin by proposal order, L-T4).
#[derive(Clone, Debug, Eq, PartialEq)]
struct CallSlot {
    id: EffectId,
    call: ProposedToolCall,
    done: bool,
    awaiting_approval: Option<SubmissionId>,
}

/// Canonical Q8 function-tool input.
#[derive(serde::Deserialize)]
struct RecallArgs {
    script: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecallWait {
    Pages,
    Ask,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RecallPending {
    id: EffectId,
    wait: RecallWait,
}

/// One live Q4 continuation. The query id is its ordinary tool-call id;
/// pending IO/sub-model work uses fresh child effect ids.
struct RecallQuery {
    id: EffectId,
    executor: Executor,
    pending: Vec<RecallPending>,
}

/// The engine.
pub struct Engine {
    config: SessionConfig,
    phase: Phase,
    epoch: TurnEpoch,
    next_seq: u64,
    next_effect: u64,
    next_request: u64,
    persisted: u64,
    /// Exactly-once terminal ledger for the current epoch.
    finished: Vec<EffectId>,
    calls: Vec<CallSlot>,
    recalls: Vec<RecallQuery>,
    tail: Vec<TailItem>,
    last_user_query: String,
    summary: String,
    identical_run: u32,
    last_call_shape: Option<(String, String)>,
    /// Compaction scheduling (consulted at turn boundaries).
    pub compaction: CompactionState,
    /// Token accounting (C8).
    pub tokens: TokenAccounting,
}

impl Engine {
    /// A fresh engine for one session. `header_seq_base` is the next free
    /// sequence after the header record the driver already appended.
    #[must_use]
    pub fn new(config: SessionConfig, header_seq_base: u64) -> Self {
        Self {
            config,
            phase: Phase::Idle,
            epoch: TurnEpoch(0),
            next_seq: header_seq_base,
            next_effect: 1,
            next_request: 1,
            persisted: 0,
            finished: Vec::new(),
            calls: Vec::new(),
            recalls: Vec::new(),
            tail: Vec::new(),
            last_user_query: String::new(),
            summary: String::new(),
            identical_run: 0,
            last_call_shape: None,
            compaction: CompactionState::default(),
            tokens: TokenAccounting::default(),
        }
    }

    /// Reconstructs an idle engine from a validated, durably persisted log.
    ///
    /// Configuration patches are applied in record order, while sequence,
    /// effect, request/submission, and turn-epoch namespaces are seeded above
    /// every observable persisted value. Opaque effect-outcome payloads expose
    /// identity only through their record's `txn` field.
    ///
    /// KC0 deliberately resumes between turns: replay does not reconstruct a
    /// half-finished turn, publish replayed events, or commit replayed records.
    /// The startup scanner remains responsible for schema, checksum, stream-
    /// lifecycle, and crash-repair validation before calling this constructor.
    ///
    /// # Errors
    ///
    /// Returns [`ResumeError::MissingHeader`] when the first record is absent
    /// or its kind/payload is not a header. Returns another [`ResumeError`]
    /// when a persisted maximum cannot be incremented without reusing or
    /// wrapping an identifier.
    pub fn resume(mut base_config: SessionConfig, records: &[Record]) -> Result<Self, ResumeError> {
        if !matches!(
            records.first(),
            Some(Record {
                kind: RecordKind::Header,
                payload: RecordPayload::Header(_),
                ..
            })
        ) {
            return Err(ResumeError::MissingHeader);
        }

        let mut maxima = ReplayMaxima::default();
        for record in records {
            maxima.observe_record(record);
            if let RecordPayload::ConfigPatch(patch) = &record.payload {
                base_config.apply(patch.clone());
            }
        }

        let next_seq = maxima
            .seq
            .checked_add(1)
            .ok_or(ResumeError::SequenceExhausted)?;
        let next_effect = maxima
            .effect
            .checked_add(1)
            .ok_or(ResumeError::EffectIdExhausted)?;
        let next_request = maxima
            .submission
            .checked_add(1)
            .ok_or(ResumeError::SubmissionIdExhausted)?;
        if maxima.epoch == u64::MAX {
            return Err(ResumeError::TurnEpochExhausted);
        }

        let mut engine = Self::new(base_config, next_seq);
        engine.epoch = TurnEpoch(maxima.epoch);
        engine.next_effect = next_effect;
        engine.next_request = next_request;
        engine.persisted = maxima.seq;
        Ok(engine)
    }

    /// Handles one input, returning the bounded action batch.
    pub fn handle(&mut self, input: CoreInput) -> Transition {
        let mut t = Transition::default();
        match input {
            CoreInput::ClientOp(submission) => self.on_op(submission, &mut t),
            CoreInput::EffectFinished {
                id,
                epoch,
                terminal,
            } => self.on_finished(id, epoch, terminal, &mut t),
            CoreInput::Persisted { up_to_seq } => {
                self.persisted = self.persisted.max(up_to_seq);
            }
            CoreInput::PersistFailed { at_seq, message } => {
                // Cancel while the phase still names the pending effects;
                // only then latch Failed (ordering bug caught by test).
                self.cancel_all(&mut t);
                self.phase = Phase::Failed;
                t.actions
                    .push(CoreAction::Publish(Event::Error(ErrorEvent::new(
                        ErrorCode::StoreIo,
                        format!("append failed at seq {at_seq}: {message}"),
                        None,
                    ))));
            }
        }
        t
    }

    /// The durability watermark last reported by the appender.
    #[must_use]
    pub fn persisted(&self) -> u64 {
        self.persisted
    }

    /// Current turn epoch (driver stamps effect completions with this).
    #[must_use]
    pub fn epoch(&self) -> TurnEpoch {
        self.epoch
    }

    /// The effective replayable session configuration.
    #[must_use]
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    fn on_op(&mut self, submission: Submission, t: &mut Transition) {
        if matches!(self.phase, Phase::Failed) {
            return;
        }
        self.commit(
            RecordKind::AcceptedOp,
            None,
            RecordPayload::AcceptedOp(submission.clone()),
            t,
        );
        match submission.op {
            Op::UserInput { text } => self.on_user_input(text, Some(submission.id), t),
            Op::Interject { text } => {
                self.tail.push(TailItem::Message(format!("[user] {text}")));
            }
            Op::Approve { request, verdict } => self.on_approval(request, verdict, t),
            Op::Interrupt => self.on_interrupt(t),
            Op::ConfigPatch { patch } => {
                self.commit(
                    RecordKind::ConfigPatch,
                    None,
                    RecordPayload::ConfigPatch(patch.clone()),
                    t,
                );
                self.config.apply(patch);
                self.compaction.reset_breaker();
            }
            Op::Shutdown => {
                self.publish(Event::ShuttingDown, t);
                self.cancel_all(t);
                self.phase = Phase::Draining;
            }
            // Unknown future ops (non_exhaustive wire enum): the accepted-op
            // record above already preserved it; acting on it would guess.
            _ => {}
        }
    }

    fn on_user_input(
        &mut self,
        text: String,
        correlates: Option<SubmissionId>,
        t: &mut Transition,
    ) {
        if !matches!(self.phase, Phase::Idle) {
            // Mid-turn user input is treated as an interjection (SPEC's
            // no-deferral rule): it joins the tail and reaches the model at
            // the next sampling point.
            self.tail.push(TailItem::Message(format!("[user] {text}")));
            return;
        }
        self.epoch = TurnEpoch(self.epoch.0 + 1);
        self.finished.clear();
        self.identical_run = 0;
        self.last_call_shape = None;
        self.last_user_query = text;
        self.publish(
            Event::TurnStarted {
                epoch: self.epoch,
                correlates,
            },
            t,
        );
        self.start_model_call(t);
    }

    fn on_interrupt(&mut self, t: &mut Transition) {
        if matches!(self.phase, Phase::Idle | Phase::Draining | Phase::Failed) {
            return;
        }
        self.cancel_all(t);
        self.publish(
            Event::TurnEnded {
                epoch: self.epoch,
                reason: TurnEnd::Interrupted,
            },
            t,
        );
        self.phase = Phase::Idle;
    }

    fn on_approval(&mut self, request: SubmissionId, verdict: ApprovalVerdict, t: &mut Transition) {
        let Some(slot) = self
            .calls
            .iter_mut()
            .find(|s| s.awaiting_approval == Some(request))
        else {
            return;
        };
        slot.awaiting_approval = None;
        // Unknown future verdicts fail closed: anything but Approve denies.
        if verdict == ApprovalVerdict::Approve {
            let id = slot.id;
            let epoch = self.epoch;
            let call = slot.call.clone();
            self.publish(Event::ToolStarted { call: id }, t);
            if call.name == "recall" {
                self.start_recall(id, &call.args_json, true, t);
            } else {
                let _ = epoch;
                self.start_effect(id, EffectSpec::Tool(call), t);
            }
        } else {
            let id = slot.id;
            slot.done = true;
            self.finished.push(id);
            self.publish(
                Event::ToolTerminal {
                    call: id,
                    outcome: ToolOutcome::Denied,
                },
                t,
            );
            self.tail.push(TailItem::ToolResult {
                call: id,
                text: String::from("[denied by operator]"),
            });
            self.maybe_resample(t);
        }
    }

    fn on_finished(
        &mut self,
        id: EffectId,
        epoch: TurnEpoch,
        terminal: EffectTerminal,
        t: &mut Transition,
    ) {
        // Exactly-once ledger + epoch discipline (L-T1/L-T2): late,
        // duplicate, or stale-epoch completions are dropped with a trace and
        // do NOT close a stream (the stream this id started, if any, was
        // already terminalized when its real completion arrived).
        // A completion is only honored if this id names an effect the engine
        // actually started and still owns (an in-flight model call, a live
        // tool slot, or a live recall child) AND its epoch matches AND it is
        // not already finished. Anything else — stale epoch, duplicate, or an
        // id that was never started (spurious/wrong-kind completion) — is
        // dropped with a trace and NEVER terminalizes a stream (review input
        // 20 #4): terminalizing an unowned id would persist a StreamTerminal
        // with no matching StreamStarted and corrupt replay.
        let owns_recall = self
            .recalls
            .iter()
            .any(|query| query.pending.iter().any(|pending| pending.id == id));
        let owns_model = matches!(self.phase, Phase::AwaitingModel(m) if m == id);
        let owns_tool = self.calls.iter().any(|slot| slot.id == id && !slot.done);
        if epoch != self.epoch
            || self.finished.contains(&id)
            || !(owns_recall || owns_model || owns_tool)
        {
            self.commit_dropped_completion(id, epoch, "unowned or stale completion", t);
            return;
        }
        // The owned effect reached its real terminal: close its lifecycle
        // stream (paired with the StreamStarted from start_effect, SPEC S3).
        self.commit_stream_terminal(id, t);
        if owns_recall {
            self.on_recall_effect_terminal(id, terminal, t);
            return;
        }
        match terminal {
            EffectTerminal::Model(outcome) => self.on_model_terminal(id, outcome, t),
            EffectTerminal::Tool { outcome, output } => {
                self.on_tool_terminal(id, outcome, &output, t);
            }
            EffectTerminal::Pages(_) | EffectTerminal::Ask(_) => {
                // An owned model/tool effect delivered a recall-child terminal
                // kind — a driver contract violation, not a normal path. The
                // stream is already closed above; surface it as an internal
                // error rather than silently dropping.
                self.finished.push(id);
                self.publish(
                    Event::Error(ErrorEvent::new(
                        ErrorCode::Internal,
                        String::from("wrong terminal kind for a non-recall effect"),
                        None,
                    )),
                    t,
                );
            }
            EffectTerminal::Failed { error, message } => {
                self.finished.push(id);
                self.publish(Event::Error(ErrorEvent::new(error, message, None)), t);
                // A failed root model call ends the turn; a failed tool
                // resolves its slot like a failed tool outcome.
                if matches!(self.phase, Phase::AwaitingModel(m) if m == id) {
                    self.end_turn(TurnEnd::Failed, t);
                } else if let Some(slot) = self.calls.iter_mut().find(|s| s.id == id) {
                    slot.done = true;
                    self.tail.push(TailItem::ToolResult {
                        call: id,
                        text: String::from("[tool failed]"),
                    });
                    self.maybe_resample(t);
                }
            }
        }
    }

    fn on_model_terminal(&mut self, id: EffectId, outcome: ModelOutcome, t: &mut Transition) {
        if !matches!(self.phase, Phase::AwaitingModel(m) if m == id) {
            return;
        }
        self.finished.push(id);
        if let Some(usage) = outcome.usage {
            self.tokens
                .record_provider_usage(usage.prompt_tokens, usage.prompt_bytes);
        }
        if !outcome.text.is_empty() {
            self.tail
                .push(TailItem::Message(format!("[assistant] {}", outcome.text)));
        }
        if outcome.tool_calls.is_empty() {
            self.end_turn(TurnEnd::Completed, t);
            return;
        }
        // Stationarity guard (L-T3): a run of identical proposals ends the
        // turn instead of looping forever.
        let shape = (
            outcome.tool_calls[0].name.clone(),
            outcome.tool_calls[0].args_json.clone(),
        );
        if self.last_call_shape.as_ref() == Some(&shape) {
            self.identical_run += 1;
        } else {
            self.identical_run = 1;
            self.last_call_shape = Some(shape);
        }
        if self.identical_run >= u32::from(self.config.stationarity.identical_calls) {
            self.publish(
                Event::Error(ErrorEvent::new(
                    ErrorCode::Internal,
                    String::from("stationarity guard: identical tool-call run"),
                    None,
                )),
                t,
            );
            self.end_turn(TurnEnd::Failed, t);
            return;
        }
        self.phase = Phase::RunningTools;
        self.calls.clear();
        self.dispatch_tool_calls(outcome.tool_calls, t);
        self.maybe_resample(t);
    }

    /// Routes proposed calls through their approval policy (L-T4:
    /// approval serial, execution concurrent).
    fn dispatch_tool_calls(&mut self, mut calls: Vec<ProposedToolCall>, t: &mut Transition) {
        // Bound the per-turn fan-out so the transition stays small
        // (review input 19 #9). Excess proposals are dropped with a trace.
        if calls.len() > MAX_TOOL_CALLS_PER_TURN {
            let dropped = calls.len() - MAX_TOOL_CALLS_PER_TURN;
            calls.truncate(MAX_TOOL_CALLS_PER_TURN);
            self.commit(
                RecordKind::EffectOutcome,
                None,
                RecordPayload::EffectOutcome(
                    format!("dropped {dropped} tool calls over the per-turn limit").into_bytes(),
                ),
                t,
            );
        }
        for call in calls {
            let call_id = self.fresh_effect();
            self.tail.push(TailItem::ToolCall {
                call: call_id,
                text: format!("{} {}", call.name, call.args_json),
            });
            self.publish(
                Event::ToolProposed {
                    call: call_id,
                    name: call.name.clone(),
                    args_json: call.args_json.clone(),
                },
                t,
            );
            let policy = self
                .config
                .approval_defaults
                .get(&call.name)
                .copied()
                .unwrap_or(ApprovalPolicy::Auto);
            match policy {
                ApprovalPolicy::Auto => {
                    self.calls.push(CallSlot {
                        id: call_id,
                        call: call.clone(),
                        done: false,
                        awaiting_approval: None,
                    });
                    self.publish(Event::ToolStarted { call: call_id }, t);
                    if call.name == "recall" {
                        self.start_recall(call_id, &call.args_json, false, t);
                    } else {
                        self.start_effect(call_id, EffectSpec::Tool(call), t);
                    }
                }
                ApprovalPolicy::Deny => {
                    self.finished.push(call_id);
                    self.calls.push(CallSlot {
                        id: call_id,
                        call,
                        done: true,
                        awaiting_approval: None,
                    });
                    self.publish(
                        Event::ToolTerminal {
                            call: call_id,
                            outcome: ToolOutcome::Denied,
                        },
                        t,
                    );
                    self.tail.push(TailItem::ToolResult {
                        call: call_id,
                        text: String::from("[denied by policy]"),
                    });
                }
                // Ask, and any unknown future policy, fail toward asking.
                _ => {
                    let request = SubmissionId(self.next_request);
                    self.next_request += 1;
                    self.calls.push(CallSlot {
                        id: call_id,
                        call: call.clone(),
                        done: false,
                        awaiting_approval: Some(request),
                    });
                    self.publish(
                        Event::ApprovalRequested {
                            request,
                            call: call_id,
                            description: format!("{} {}", call.name, call.args_json),
                        },
                        t,
                    );
                }
            }
        }
    }

    fn on_tool_terminal(
        &mut self,
        id: EffectId,
        outcome: ToolOutcome,
        output: &str,
        t: &mut Transition,
    ) {
        self.resolve_tool_terminal(id, outcome, output, true, t);
    }

    fn resolve_tool_terminal(
        &mut self,
        id: EffectId,
        outcome: ToolOutcome,
        output: &str,
        resample: bool,
        t: &mut Transition,
    ) {
        let Some(index) = self.calls.iter().position(|s| s.id == id && !s.done) else {
            return;
        };
        self.finished.push(id);
        // Full output goes to the log; the window gets the capped excerpt
        // with the log pointer (Q3 reversible offload).
        let full_seq = self.next_seq;
        self.commit(
            RecordKind::EffectOutcome,
            Some(id),
            RecordPayload::EffectOutcome(Vec::from(output.as_bytes())),
            t,
        );
        // Reserve room for the truncation annotation so the total surfaced
        // value stays within the declared tool-result budget (review input
        // 19 #14): the excerpt is capped to budget minus the annotation's
        // maximum length.
        let excerpt_budget = self
            .config
            .budgets
            .tool_result_bytes
            .saturating_sub(ANNOTATION_RESERVE);
        let capped = Capped::<ToolResultCap>::head_tail(output, excerpt_budget, Some(full_seq));
        let mut text = String::from(capped.as_str());
        if let Some(trunc) = capped.truncation() {
            use core::fmt::Write as _;
            let shown = text.len();
            let _ = write!(
                text,
                "\n[truncated: {shown} of {} bytes; full output at log seq {full_seq}]",
                trunc.original_bytes
            );
        }
        // Never exceed the declared budget, annotation included.
        debug_assert!(text.len() <= self.config.budgets.tool_result_bytes as usize);
        self.calls[index].done = true;
        self.tail.push(TailItem::ToolResult { call: id, text });
        self.publish(Event::ToolTerminal { call: id, outcome }, t);
        if resample {
            self.maybe_resample(t);
        }
    }

    /// Starts the Q8 recall continuation in place of an ordinary driver
    /// tool effect. Lowering failures and session continuation-cap failures
    /// resolve through the ordinary tool terminal path.
    fn start_recall(&mut self, id: EffectId, args_json: &str, resample: bool, t: &mut Transition) {
        if self.recalls.len() >= usize::from(self.config.budgets.suspended_queries) {
            let message = String::from("verb_error{verb:recall,cause:budget}");
            self.resolve_tool_terminal(
                id,
                ToolOutcome::Failed {
                    message: message.clone(),
                },
                &message,
                resample,
                t,
            );
            return;
        }

        let Ok(args) = serde_json::from_str::<RecallArgs>(args_json) else {
            let message = String::from("verb_error{verb:recall,cause:parse}");
            self.resolve_tool_terminal(
                id,
                ToolOutcome::Failed {
                    message: message.clone(),
                },
                &message,
                resample,
                t,
            );
            return;
        };
        let query = lower_script(&args.script);
        if let Some(error) = query.iter().find_map(|binding| match &binding.value {
            BoundValue::Error(error) => Some(error),
            BoundValue::Instr(_) => None,
        }) {
            let message = format!(
                "verb_error{{verb:{},cause:{}}}",
                error.verb,
                verb_error_name(error.cause)
            );
            self.resolve_tool_terminal(
                id,
                ToolOutcome::Failed {
                    message: message.clone(),
                },
                &message,
                resample,
                t,
            );
            return;
        }

        let mut continuation = RecallQuery {
            id,
            executor: Executor::new(query, self.config.budgets),
            pending: Vec::new(),
        };
        let outcome = continuation.executor.step();
        self.advance_recall(continuation, outcome, resample, t);
    }

    fn on_recall_effect_terminal(
        &mut self,
        child: EffectId,
        terminal: EffectTerminal,
        t: &mut Transition,
    ) {
        let Some(query_index) = self
            .recalls
            .iter()
            .position(|query| query.pending.iter().any(|pending| pending.id == child))
        else {
            return;
        };
        let mut query = self.recalls.remove(query_index);
        let pending_index = query
            .pending
            .iter()
            .position(|pending| pending.id == child)
            .expect("pending child was located above");
        let pending = query.pending.remove(pending_index);
        self.finished.push(child);

        let outcome = match (pending.wait, terminal) {
            (RecallWait::Pages, EffectTerminal::Pages(page)) => query.executor.provide_pages(page),
            (RecallWait::Ask, EffectTerminal::Ask(results)) => query.executor.provide_ask(results),
            (_, EffectTerminal::Failed { message, .. }) => {
                self.cancel_recall_children(&query, t);
                self.resolve_tool_terminal(
                    query.id,
                    ToolOutcome::Failed {
                        message: message.clone(),
                    },
                    &message,
                    true,
                    t,
                );
                return;
            }
            _ => StepOutcome::Failed {
                cause: VerbErrorCause::Parse,
            },
        };
        self.advance_recall(query, outcome, true, t);
    }

    fn advance_recall(
        &mut self,
        mut query: RecallQuery,
        mut outcome: StepOutcome,
        resample: bool,
        t: &mut Transition,
    ) {
        loop {
            match outcome {
                StepOutcome::NeedPages(request) => {
                    let child = self.fresh_effect();
                    query.pending.push(RecallPending {
                        id: child,
                        wait: RecallWait::Pages,
                    });
                    self.start_effect(
                        child,
                        EffectSpec::StoreReadPage {
                            sel: request.sel,
                            cursor: request.cursor,
                        },
                        t,
                    );
                    self.recalls.push(query);
                    return;
                }
                StepOutcome::NeedAsk(requests) => {
                    if requests.is_empty() {
                        outcome = StepOutcome::Failed {
                            cause: VerbErrorCause::Parse,
                        };
                        continue;
                    }
                    for request in requests {
                        let child = self.fresh_effect();
                        query.pending.push(RecallPending {
                            id: child,
                            wait: RecallWait::Ask,
                        });
                        self.start_effect(
                            child,
                            EffectSpec::SubModel {
                                requests: alloc::vec![request],
                            },
                            t,
                        );
                    }
                    self.recalls.push(query);
                    return;
                }
                StepOutcome::AwaitingMore => {
                    if query.pending.is_empty() {
                        outcome = StepOutcome::Failed {
                            cause: VerbErrorCause::Parse,
                        };
                        continue;
                    }
                    self.recalls.push(query);
                    return;
                }
                StepOutcome::Line { slot, bound } => {
                    self.publish(
                        Event::QueryTrace {
                            query: query.id,
                            line: u16::try_from(slot).unwrap_or(u16::MAX),
                            note: format!("bound {}", bound_name(&bound)),
                        },
                        t,
                    );
                    outcome = query.executor.step();
                }
                StepOutcome::Done { answer } => {
                    self.cancel_recall_children(&query, t);
                    self.resolve_tool_terminal(
                        query.id,
                        ToolOutcome::Succeeded,
                        &answer,
                        resample,
                        t,
                    );
                    return;
                }
                StepOutcome::Failed { cause } => {
                    self.cancel_recall_children(&query, t);
                    let message =
                        format!("verb_error{{verb:recall,cause:{}}}", verb_error_name(cause));
                    self.resolve_tool_terminal(
                        query.id,
                        ToolOutcome::Failed {
                            message: message.clone(),
                        },
                        &message,
                        resample,
                        t,
                    );
                    return;
                }
            }
        }
    }

    fn cancel_recall_children(&mut self, query: &RecallQuery, t: &mut Transition) {
        for pending in &query.pending {
            if !self.finished.contains(&pending.id) {
                self.finished.push(pending.id);
                t.actions.push(CoreAction::CancelEffect { id: pending.id });
            }
        }
    }

    fn maybe_resample(&mut self, t: &mut Transition) {
        if !matches!(self.phase, Phase::RunningTools) {
            return;
        }
        if self.calls.iter().all(|s| s.done)
            && self.calls.iter().all(|s| s.awaiting_approval.is_none())
        {
            self.calls.clear();
            self.start_model_call(t);
        }
    }

    fn start_model_call(&mut self, t: &mut Transition) {
        let id = self.fresh_effect();
        self.phase = Phase::AwaitingModel(id);
        let window = self.assemble_window();
        self.start_effect(id, EffectSpec::ModelCall(window), t);
    }

    fn end_turn(&mut self, reason: TurnEnd, t: &mut Transition) {
        self.publish(
            Event::TurnEnded {
                epoch: self.epoch,
                reason,
            },
            t,
        );
        self.phase = Phase::Idle;
        self.calls.clear();
        self.recalls.clear();
    }

    fn cancel_all(&mut self, t: &mut Transition) {
        if let Phase::AwaitingModel(id) = self.phase {
            t.actions.push(CoreAction::CancelEffect { id });
            // Close the cancelled effect's lifecycle stream so a crash right
            // after interrupt leaves no orphan StreamStarted that repair would
            // falsely resurrect (review input 19 #4).
            self.commit_stream_terminal(id, t);
            self.finished.push(id);
        }
        let recall_ids: Vec<EffectId> = self.recalls.iter().map(|query| query.id).collect();
        for id in &recall_ids {
            self.commit(
                RecordKind::EffectOutcome,
                Some(*id),
                RecordPayload::EffectOutcome(
                    String::from("query continuation discarded").into_bytes(),
                ),
                t,
            );
        }
        let child_ids: Vec<EffectId> = self
            .recalls
            .iter()
            .flat_map(|query| query.pending.iter().map(|pending| pending.id))
            .collect();
        for id in child_ids {
            if !self.finished.contains(&id) {
                self.finished.push(id);
                t.actions.push(CoreAction::CancelEffect { id });
                self.commit_stream_terminal(id, t);
            }
        }
        for id in &recall_ids {
            if !self.finished.contains(id) {
                self.finished.push(*id);
            }
        }
        let cancel_slots: Vec<EffectId> = self
            .calls
            .iter()
            .filter(|slot| {
                !slot.done && slot.awaiting_approval.is_none() && !recall_ids.contains(&slot.id)
            })
            .map(|slot| slot.id)
            .collect();
        for id in cancel_slots {
            t.actions.push(CoreAction::CancelEffect { id });
            self.commit_stream_terminal(id, t);
            self.finished.push(id);
        }
        self.recalls.clear();
        self.calls.clear();
    }

    fn commit_dropped_completion(
        &mut self,
        id: EffectId,
        epoch: TurnEpoch,
        reason: &str,
        t: &mut Transition,
    ) {
        self.commit(
            RecordKind::EffectOutcome,
            Some(id),
            RecordPayload::EffectOutcome(
                format!(
                    "dropped completion: {reason}; effect {} epoch {}",
                    id.0, epoch.0
                )
                .into_bytes(),
            ),
            t,
        );
    }

    /// Assembles the current window (SPEC C10 recipe). Infallible because
    /// the engine's own tail construction preserves call/result pairing;
    /// the constructor still checks it (defense in depth for G6).
    fn assemble_window(&self) -> WindowLayout {
        let mut reminders = Vec::new();
        reminders.push(String::from(prompts::resolve(
            TemplateId::RlmReminder,
            &self.config,
        )));
        WindowLayout::new(
            String::from(prompts::resolve(TemplateId::System, &self.config)),
            String::new(),
            String::new(),
            self.last_user_query.clone(),
            self.tail.clone(),
            self.summary.clone(),
            reminders,
        )
        .unwrap_or_else(|_| {
            // Unreachable by construction; fall back to an empty tail
            // rather than panicking inside the reactor (no-panic law).
            WindowLayout::new(
                String::from(prompts::resolve(TemplateId::System, &self.config)),
                String::new(),
                String::new(),
                self.last_user_query.clone(),
                Vec::new(),
                self.summary.clone(),
                Vec::new(),
            )
            .expect("empty tail is always atomic")
        })
    }

    fn fresh_effect(&mut self) -> EffectId {
        let id = EffectId(self.next_effect);
        self.next_effect += 1;
        id
    }

    /// Dispatches an effect through the single choke point: it commits a
    /// `StreamStarted` lifecycle record keyed by the effect id BEFORE the
    /// `StartEffect` action, so a crash between here and the effect's
    /// terminal leaves an orphan `StreamStarted` that the appender's startup
    /// scanner repairs into an `aborted_by_crash` terminal (SPEC S3; review
    /// input 19 #3). Every model/tool/RLM effect starts here.
    fn start_effect(&mut self, id: EffectId, spec: EffectSpec, t: &mut Transition) {
        self.commit(
            RecordKind::StreamStarted,
            Some(id),
            RecordPayload::StreamStarted(Vec::new()),
            t,
        );
        t.actions.push(CoreAction::StartEffect {
            id,
            epoch: self.epoch,
            spec,
        });
    }

    /// Commits the `StreamTerminal` lifecycle record that closes an effect's
    /// transaction (paired with the `StreamStarted` from [`Self::start_effect`]).
    /// Call exactly once per effect that reached a terminal, alongside its
    /// `EffectOutcome`. The persisted `Started -> Terminal` pair is what makes
    /// the exactly-once effect lifecycle discoverable on replay (SPEC S3).
    fn commit_stream_terminal(&mut self, id: EffectId, t: &mut Transition) {
        self.commit(
            RecordKind::StreamTerminal,
            Some(id),
            RecordPayload::StreamTerminal(Vec::new()),
            t,
        );
    }

    fn publish(&mut self, event: Event, t: &mut Transition) {
        self.commit(
            RecordKind::EmittedEvent,
            None,
            RecordPayload::EmittedEvent(event.clone()),
            t,
        );
        t.actions.push(CoreAction::Publish(event));
    }

    fn commit(
        &mut self,
        kind: RecordKind,
        txn: Option<EffectId>,
        payload: RecordPayload,
        t: &mut Transition,
    ) {
        let seq = self.next_seq;
        self.next_seq += 1;
        match Record::new(seq, kind, txn, self.epoch, payload) {
            Ok(record) => match t.actions.last_mut() {
                Some(CoreAction::Commit(records)) => records.push(record),
                _ => t.actions.push(CoreAction::Commit(alloc::vec![record])),
            },
            Err(_) => {
                // Kind/payload mismatch is a bug, not a runtime condition.
                debug_assert!(false, "record kind/payload mismatch");
            }
        }
    }
}

fn verb_error_name(cause: VerbErrorCause) -> &'static str {
    match cause {
        VerbErrorCause::BadRef => "bad_ref",
        VerbErrorCause::BadRange => "bad_range",
        VerbErrorCause::BadFlag => "bad_flag",
        VerbErrorCause::Parse => "parse",
        VerbErrorCause::Budget => "budget",
        _ => "unknown",
    }
}

fn bound_name(bound: &Bound) -> &'static str {
    match bound {
        Bound::Records(_) => "records",
        Bound::Chunks(_) => "chunks",
        Bound::Count(_) => "count",
        Bound::Digest(_) => "digest",
        Bound::DigestList(_) => "digest_list",
        Bound::Error(_) => "verb_error",
    }
}
