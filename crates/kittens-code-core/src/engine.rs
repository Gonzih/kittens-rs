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

use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::error::{ErrorCode, ErrorEvent};
use kittens_code_protocol::event::{Event, ToolOutcome, TurnEnd};
use kittens_code_protocol::ids::{EffectId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use kittens_code_protocol::policy::{ApprovalPolicy, ApprovalVerdict};

use crate::caps::{Capped, ToolResult as ToolResultCap};
use crate::compact::CompactionState;
use crate::prompts::{self, TemplateId};
use crate::record::{Record, RecordKind, RecordPayload};
use crate::tokens::TokenAccounting;
use crate::window::{TailItem, WindowLayout};

/// Upper bound on actions per transition (L-A2: bounded owned batches).
pub const MAX_ACTIONS: usize = 64;

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

/// One in-flight tool call slot (rejoin by proposal order, L-T4).
#[derive(Clone, Debug, Eq, PartialEq)]
struct CallSlot {
    id: EffectId,
    call: ProposedToolCall,
    done: bool,
    awaiting_approval: Option<SubmissionId>,
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
            tail: Vec::new(),
            last_user_query: String::new(),
            summary: String::new(),
            identical_run: 0,
            last_call_shape: None,
            compaction: CompactionState::default(),
            tokens: TokenAccounting::default(),
        }
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
        debug_assert!(t.actions.len() <= MAX_ACTIONS);
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
                self.phase = Phase::Draining;
                self.publish(Event::ShuttingDown, t);
                self.cancel_all(t);
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
            let spec = EffectSpec::Tool(slot.call.clone());
            self.publish(Event::ToolStarted { call: id }, t);
            t.actions.push(CoreAction::StartEffect { id, epoch, spec });
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
        // duplicate, or stale-epoch completions are dropped with a trace.
        if epoch != self.epoch || self.finished.contains(&id) {
            self.commit(
                RecordKind::EffectOutcome,
                Some(id),
                RecordPayload::EffectOutcome(
                    format!("dropped completion: effect {} epoch {}", id.0, epoch.0).into_bytes(),
                ),
                t,
            );
            return;
        }
        match terminal {
            EffectTerminal::Model(outcome) => self.on_model_terminal(id, outcome, t),
            EffectTerminal::Tool { outcome, output } => {
                self.on_tool_terminal(id, outcome, &output, t);
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
    fn dispatch_tool_calls(&mut self, calls: Vec<ProposedToolCall>, t: &mut Transition) {
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
                    t.actions.push(CoreAction::StartEffect {
                        id: call_id,
                        epoch: self.epoch,
                        spec: EffectSpec::Tool(call),
                    });
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
        let capped = Capped::<ToolResultCap>::head_tail(
            output,
            self.config.budgets.tool_result_bytes,
            Some(full_seq),
        );
        let mut text = String::from(capped.as_str());
        if let Some(trunc) = capped.truncation() {
            use core::fmt::Write as _;
            let shown = text.len();
            let _ = write!(
                text,
                "\n[truncated: {shown} of {} bytes shown; full output at log seq {full_seq}]",
                trunc.original_bytes
            );
        }
        self.calls[index].done = true;
        self.tail.push(TailItem::ToolResult { call: id, text });
        self.publish(Event::ToolTerminal { call: id, outcome }, t);
        self.maybe_resample(t);
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
        t.actions.push(CoreAction::StartEffect {
            id,
            epoch: self.epoch,
            spec: EffectSpec::ModelCall(window),
        });
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
    }

    fn cancel_all(&mut self, t: &mut Transition) {
        if let Phase::AwaitingModel(id) = self.phase {
            t.actions.push(CoreAction::CancelEffect { id });
        }
        for slot in &self.calls {
            if !slot.done && slot.awaiting_approval.is_none() {
                t.actions.push(CoreAction::CancelEffect { id: slot.id });
            }
        }
        self.calls.clear();
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
