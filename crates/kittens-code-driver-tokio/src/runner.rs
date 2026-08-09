//! The driver run loop (SPEC L-D1, L-A2b, L-A3).
//!
//! KC0 uses the owned-task + funnel topology directly on Tokio primitives.
//! Client ops and effect completions arrive over one bounded channel; each
//! is fed to `core.handle`, and the resulting bounded action batch is
//! discharged as a whole (L-A2b): commits go to the single appender first,
//! and events publish only after their covering durability watermark
//! (L-A3). Effects are spawned as owned tasks that funnel their terminals
//! back through the same completion channel.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use std::{fs::File, io::BufRead, io::BufReader, path::Path};

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine, ModelOutcome, ResumeError,
};
use kittens_code_core::record::Record;
use kittens_code_core::rlm::exec::{AskResult, Page, PageRecord};
use kittens_code_core::rlm::ir::{RangeUnit, Sel};
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{EffectId, TurnEpoch};
use kittens_code_protocol::op::Submission;
use tokio::sync::mpsc;

use crate::appender::{Appender, OpenError};
use crate::model::ModelClient;
use crate::tools;

/// Items funneled into the run loop (the single completion channel).
enum Wake {
    /// A client submission arrived.
    Op(Submission),
    /// An effect finished (model or tool).
    Finished {
        id: EffectId,
        epoch: TurnEpoch,
        terminal: EffectTerminal,
    },
}

/// Drives one session to quiescence or shutdown against a model client and
/// a workspace root. Returns every event published, in order (the headless
/// driver's output; a streaming frontend would forward these live).
pub struct Runner {
    engine: Engine,
    appender: Appender,
    model: Arc<dyn ModelClient>,
    root: PathBuf,
    tx: mpsc::UnboundedSender<Wake>,
    rx: mpsc::UnboundedReceiver<Wake>,
    published: Vec<Event>,
    outstanding: u32,
    watermark: u64,
    failed: bool,
}

/// Why opening a session failed.
#[derive(Debug)]
pub enum OpenSessionError {
    /// The log could not be opened or crash-repaired.
    Open(OpenError),
    /// The replayed records could not seed the engine.
    Resume(ResumeError),
}

impl Runner {
    /// Wires a runner over an opened appender and a model client.
    #[must_use]
    pub fn new(
        engine: Engine,
        appender: Appender,
        model: Arc<dyn ModelClient>,
        root: PathBuf,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            engine,
            appender,
            model,
            root,
            tx,
            rx,
            published: Vec::new(),
            outstanding: 0,
            watermark: 0,
            failed: false,
        }
    }

    /// Opens (or creates) the session log and wires a runner whose engine is
    /// resumed from the replayed records (SPEC S2 resume-as-replay). On a
    /// fresh log this is equivalent to `new` over an engine at the header's
    /// base sequence; on an existing log the engine's counters are seeded
    /// above every persisted value so no id is reused.
    ///
    /// # Errors
    ///
    /// [`OpenError`] if the log cannot be opened or repaired, or a
    /// [`ResumeError`] if the replayed records cannot seed the engine.
    pub fn open(
        log_path: &std::path::Path,
        fresh_header: Option<Record>,
        base_config: SessionConfig,
        model: Arc<dyn ModelClient>,
        root: PathBuf,
    ) -> Result<Self, OpenSessionError> {
        let (appender, replay) =
            Appender::open(log_path, fresh_header).map_err(OpenSessionError::Open)?;
        let engine = Engine::resume(base_config, &replay).map_err(OpenSessionError::Resume)?;
        Ok(Self::new(engine, appender, model, root))
    }

    /// Submits a client op into the funnel.
    pub fn submit(&self, submission: Submission) {
        let _ = self.tx.send(Wake::Op(submission));
    }

    /// Runs until the session is quiescent: no queued wakes and no
    /// outstanding effect. Every submitted op is processed to completion,
    /// including all the model/tool round-trips it triggers. Returns the
    /// ordered event log for this drive.
    ///
    /// Quiescence is tracked by an outstanding-effect counter: a
    /// `StartEffect` increments it, a `Finished` wake decrements it. The
    /// loop ends when the counter is zero and the channel is empty, so a
    /// still-running tool always re-wakes the loop before it can exit.
    pub async fn run_to_idle(&mut self) -> &[Event] {
        loop {
            let wake = match self.rx.try_recv() {
                Ok(wake) => wake,
                Err(mpsc::error::TryRecvError::Empty) if self.outstanding == 0 => break,
                Err(mpsc::error::TryRecvError::Empty) => match self.rx.recv().await {
                    Some(wake) => wake,
                    None => break,
                },
                Err(mpsc::error::TryRecvError::Disconnected) => break,
            };
            let transition = match wake {
                Wake::Op(submission) => self.engine.handle(CoreInput::ClientOp(submission)),
                Wake::Finished {
                    id,
                    epoch,
                    terminal,
                } => {
                    self.outstanding -= 1;
                    self.engine.handle(CoreInput::EffectFinished {
                        id,
                        epoch,
                        terminal,
                    })
                }
            };
            self.discharge(transition.actions);
        }
        &self.published
    }

    /// Discharges one bounded action batch as a unit (L-A2b whole-batch
    /// dispatch): every `Commit` is appended durably first, so that
    /// `Publish` never surfaces an event whose records are not yet durable
    /// (L-A3). An append failure latches a terminal state, drops the rest of
    /// the batch, and feeds the engine exactly one `PersistFailed` whose
    /// resulting actions (cancels only, once latched) are dispatched without
    /// re-entering the failed appender — so a persistent store fault cannot
    /// recurse (review input 19 #1).
    fn discharge(&mut self, actions: Vec<CoreAction>) {
        // First pass: durably commit all records in order. On the first
        // failure, latch and stop committing.
        if !self.failed {
            for action in &actions {
                if let CoreAction::Commit(records) = action {
                    match self.appender.append(records) {
                        Ok(watermark) => self.watermark = watermark,
                        Err((at_seq, e)) => {
                            self.failed = true;
                            // One PersistFailed; its actions are cancels and
                            // a fatal error event we surface directly, never
                            // through another append.
                            let t = self.engine.handle(CoreInput::PersistFailed {
                                at_seq,
                                message: e.to_string(),
                            });
                            self.dispatch_terminal(t.actions);
                            return;
                        }
                    }
                }
            }
            // Acknowledge durability to the engine once, after the whole
            // batch is committed (not mid-loop).
            let t = self.engine.handle(CoreInput::Persisted {
                up_to_seq: self.watermark,
            });
            debug_assert!(t.actions.is_empty());
        }

        // Second pass: publish durable events and start effects in order.
        // KC0 cancellation is cooperative (a late terminal is dropped by the
        // engine's exactly-once ledger), so CancelEffect and the
        // already-handled Commit are no-ops here.
        for action in actions {
            match action {
                CoreAction::Publish(event) => self.published.push(event),
                CoreAction::StartEffect { id, epoch, spec } => {
                    if !self.failed {
                        self.outstanding += 1;
                        self.spawn_effect(id, epoch, spec);
                    }
                }
                CoreAction::Commit(_) | CoreAction::CancelEffect { .. } => {}
                other => unimplemented!("driver missing CoreAction handler: {other:?}"),
            }
        }
    }

    /// Dispatches the terminal (post-failure) action batch: it may publish
    /// the fatal error but MUST NOT append (the appender is dead) or start
    /// new effects (review input 19 #1).
    fn dispatch_terminal(&mut self, actions: Vec<CoreAction>) {
        for action in actions {
            if let CoreAction::Publish(event) = action {
                self.published.push(event);
            }
        }
    }

    fn spawn_effect(&self, id: EffectId, epoch: TurnEpoch, spec: EffectSpec) {
        let tx = self.tx.clone();
        match spec {
            EffectSpec::ModelCall(window) => {
                let model = Arc::clone(&self.model);
                tokio::spawn(async move {
                    let terminal = match model.complete(window).await {
                        Ok(outcome) => EffectTerminal::Model(outcome),
                        Err((error, message)) => EffectTerminal::Failed { error, message },
                    };
                    let _ = tx.send(Wake::Finished {
                        id,
                        epoch,
                        terminal,
                    });
                });
            }
            EffectSpec::Tool(call) => {
                let root = self.root.clone();
                tokio::spawn(async move {
                    let (outcome, output) =
                        tokio::task::spawn_blocking(move || tools::run(&root, &call))
                            .await
                            .unwrap_or_else(|e| {
                                (
                                    kittens_code_protocol::event::ToolOutcome::Failed {
                                        message: e.to_string(),
                                    },
                                    e.to_string(),
                                )
                            });
                    let _ = tx.send(Wake::Finished {
                        id,
                        epoch,
                        terminal: EffectTerminal::Tool { outcome, output },
                    });
                });
            }
            EffectSpec::StoreReadPage { sel, cursor } => {
                let path = self.appender.path().to_path_buf();
                tokio::spawn(async move {
                    let terminal = match tokio::task::spawn_blocking(move || {
                        read_store_page(&path, &sel, cursor)
                    })
                    .await
                    {
                        Ok(Ok(page)) => EffectTerminal::Pages(page),
                        Ok(Err(error)) => EffectTerminal::Failed {
                            error: kittens_code_protocol::error::ErrorCode::StoreIo,
                            message: error.to_string(),
                        },
                        Err(error) => EffectTerminal::Failed {
                            error: kittens_code_protocol::error::ErrorCode::StoreIo,
                            message: error.to_string(),
                        },
                    };
                    let _ = tx.send(Wake::Finished {
                        id,
                        epoch,
                        terminal,
                    });
                });
            }
            EffectSpec::SubModel { requests } => {
                let model = Arc::clone(&self.model);
                tokio::spawn(async move {
                    let mut results = Vec::with_capacity(requests.len());
                    let mut failure = None;
                    for request in requests {
                        let index = request.index;
                        let started = Instant::now();
                        let completion = model.complete_submodel(request).await;
                        let wall_clock_ms =
                            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        match completion {
                            Ok(outcome) => {
                                results.push(ask_result(index, outcome, wall_clock_ms));
                            }
                            Err(error) => {
                                failure = Some(error);
                                break;
                            }
                        }
                    }
                    let terminal = failure.map_or_else(
                        || EffectTerminal::Ask(results),
                        |(error, message)| EffectTerminal::Failed { error, message },
                    );
                    let _ = tx.send(Wake::Finished {
                        id,
                        epoch,
                        terminal,
                    });
                });
            }
            // Unknown future effect specs (l2 embed, timers): reaching here
            // is a configuration bug, not a runtime condition.
            other => unimplemented!("driver missing EffectSpec handler: {other:?}"),
        }
    }
}

fn ask_result(index: u32, outcome: ModelOutcome, wall_clock_ms: u64) -> AskResult {
    // Current provider usage exposes prompt tokens only. Report those as the
    // best available subcall token cost; providers without usage data
    // explicitly contribute zero.
    let tokens = outcome.usage.map_or(0, |usage| usage.prompt_tokens);
    AskResult {
        index,
        answer: outcome.text,
        wall_clock_ms,
        tokens,
    }
}

/// KC0 transcript paging: the cursor is an offset in the records selected by
/// `sel`, and each record is rendered as its canonical serialized JSON line.
fn read_store_page(path: &Path, sel: &Sel, cursor: Option<u64>) -> std::io::Result<Page> {
    const PAGE_RECORDS: usize = 64;

    let start = usize::try_from(cursor.unwrap_or(0)).unwrap_or(usize::MAX);
    let mut selected = 0usize;
    let mut records = Vec::with_capacity(PAGE_RECORDS);
    let mut byte_offset = 0u64;
    let reader = BufReader::new(File::open(path)?);
    for line in reader.lines() {
        let line = line?;
        let record: Record = serde_json::from_str(&line)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let line_start = byte_offset;
        byte_offset = byte_offset.saturating_add(line.len() as u64 + 1);
        if !record_selected(sel, &record, line_start) {
            continue;
        }
        if selected < start {
            selected += 1;
            continue;
        }
        if records.len() == PAGE_RECORDS {
            return Ok(Page {
                records,
                next_cursor: Some(u64::try_from(selected).unwrap_or(u64::MAX)),
            });
        }
        records.push(PageRecord {
            seq: record.seq,
            text: line,
        });
        selected += 1;
    }
    Ok(Page {
        records,
        next_cursor: None,
    })
}

fn record_selected(sel: &Sel, record: &Record, byte_offset: u64) -> bool {
    match sel {
        Sel::Whole => true,
        Sel::Range(range) => {
            let coordinate = match range.unit {
                RangeUnit::Turn => record.epoch.0,
                RangeUnit::Seq => record.seq,
                RangeUnit::Byte => byte_offset,
            };
            range.start <= coordinate && coordinate < range.end
        }
        // A `%N` selection is already materialized inside the executor.
        // Current lowering only sends raw store selections over this seam.
        Sel::Ref(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kittens_code_core::engine::{ProposedToolCall, Usage};
    use kittens_code_core::prompts::PROMPT_PACK_VERSION;
    use kittens_code_core::record::{LogHeader, RecordKind, RecordPayload};
    use kittens_code_core::rlm::exec::AskRequest;
    use kittens_code_core::rlm::ir::{Range, Records, Ref};
    use kittens_code_protocol::error::ErrorCode;
    use kittens_code_protocol::event::{ToolOutcome, TurnEnd};
    use kittens_code_protocol::ids::{SessionId, SubmissionId};
    use kittens_code_protocol::op::Op;

    use crate::appender::CODEC;
    use crate::model::{JailClient, JailStep};

    fn header(session: u8) -> Record {
        Record::new(
            0,
            RecordKind::Header,
            None,
            TurnEpoch(0),
            RecordPayload::Header(LogHeader {
                session_id: SessionId([session; 16]),
                parent: None,
                schema_epoch: 0,
                prompt_pack_version: PROMPT_PACK_VERSION.0,
                verb_grammar_version: [1, 0, 0],
                l3_dialect_version: [1, 0, 0],
                codec: String::from(CODEC),
                created_at: None,
            }),
        )
        .expect("header")
    }

    fn event_record(seq: u64, epoch: u64) -> Record {
        Record::new(
            seq,
            RecordKind::EmittedEvent,
            None,
            TurnEpoch(epoch),
            RecordPayload::EmittedEvent(Event::ShuttingDown),
        )
        .expect("event record")
    }

    fn submission(id: u64, op: Op) -> Submission {
        Submission {
            id: SubmissionId(id),
            op,
        }
    }

    fn open_runner(dir: &tempfile::TempDir, name: &str, model: Arc<dyn ModelClient>) -> Runner {
        let path = dir.path().join(name);
        let (appender, _) = Appender::open(&path, Some(header(1))).expect("open appender");
        Runner::new(
            Engine::new(SessionConfig::default(), appender.next_seq()),
            appender,
            model,
            dir.path().to_path_buf(),
        )
    }

    fn jail(steps: Vec<JailStep>) -> Arc<dyn ModelClient> {
        Arc::new(JailClient::new(steps))
    }

    fn success_step(text: &str) -> JailStep {
        JailStep {
            text: String::from(text),
            tool_calls: Vec::new(),
            usage: Some((7, 70)),
            fail: None,
        }
    }

    fn empty_window() -> kittens_code_core::window::WindowLayout {
        kittens_code_core::window::WindowLayout::new(
            String::new(),
            String::new(),
            String::new(),
            String::from("question"),
            Vec::new(),
            String::new(),
            Vec::new(),
        )
        .expect("empty tail")
    }

    async fn receive_finished(
        runner: &mut Runner,
    ) -> Option<(EffectId, TurnEpoch, EffectTerminal)> {
        match runner.rx.recv().await.expect("effect completion") {
            Wake::Finished {
                id,
                epoch,
                terminal,
            } => Some((id, epoch, terminal)),
            Wake::Op(_) => None,
        }
    }

    #[test]
    fn runner_open_distinguishes_appender_and_resume_failures() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = jail(Vec::new());
        let missing_parent = dir.path().join("missing/session.jsonl");
        assert!(matches!(
            Runner::open(
                &missing_parent,
                Some(header(2)),
                SessionConfig::default(),
                Arc::clone(&model),
                dir.path().to_path_buf(),
            ),
            Err(OpenSessionError::Open(OpenError::Io(_)))
        ));

        let resume_path = dir.path().join("resume-error.jsonl");
        let started = Record::new(
            1,
            RecordKind::StreamStarted,
            Some(EffectId(u64::MAX)),
            TurnEpoch(1),
            RecordPayload::StreamStarted(Vec::new()),
        )
        .expect("stream start");
        let text = [header(3), started]
            .iter()
            .map(|record| serde_json::to_string(record).expect("serialize"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&resume_path, text).expect("seed replay");
        assert!(matches!(
            Runner::open(
                &resume_path,
                None,
                SessionConfig::default(),
                model,
                dir.path().to_path_buf(),
            ),
            Err(OpenSessionError::Resume(ResumeError::EffectIdExhausted))
        ));
    }

    #[tokio::test]
    async fn persistence_failure_latches_and_dispatches_only_the_fatal_terminal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut runner = open_runner(
            &dir,
            "persist-failure.jsonl",
            jail(vec![success_step("late")]),
        );
        let actions = runner
            .engine
            .handle(CoreInput::ClientOp(submission(
                1,
                Op::UserInput {
                    text: String::from("start"),
                },
            )))
            .actions;
        assert!(
            actions
                .iter()
                .any(|action| matches!(action, CoreAction::StartEffect { .. }))
        );

        runner.appender.inject_write_failure();
        runner.discharge(actions);
        assert!(runner.failed);
        assert_eq!(runner.outstanding, 0, "failed commits start no effects");
        assert!(runner.published.iter().any(|event| matches!(
            event,
            Event::Error(error) if error.code == ErrorCode::StoreIo
        )));

        runner.discharge(vec![
            CoreAction::Publish(Event::ShuttingDown),
            CoreAction::StartEffect {
                id: EffectId(99),
                epoch: TurnEpoch(1),
                spec: EffectSpec::Tool(ProposedToolCall {
                    name: String::from("read"),
                    args_json: String::from("{}"),
                }),
            },
            CoreAction::CancelEffect { id: EffectId(99) },
            CoreAction::Commit(Vec::new()),
        ]);
        assert_eq!(runner.outstanding, 0);
        assert!(runner.published.contains(&Event::ShuttingDown));
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // Keeping every EffectSpec terminal in one audit matrix is intentional.
    async fn every_supported_effect_spec_funnels_a_terminal() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("tool.txt"), "tool output").expect("seed tool file");
        let model = jail(vec![
            success_step("root success"),
            JailStep {
                text: String::new(),
                tool_calls: Vec::new(),
                usage: None,
                fail: Some(String::from("root failure")),
            },
            success_step("sub one"),
            success_step("sub two"),
            JailStep {
                text: String::new(),
                tool_calls: Vec::new(),
                usage: None,
                fail: Some(String::from("sub failure")),
            },
        ]);
        let mut runner = open_runner(&dir, "effects.jsonl", model);
        runner
            .tx
            .send(Wake::Op(submission(
                99,
                Op::Interject {
                    text: String::from("queued control"),
                },
            )))
            .expect("queue control wake");
        assert!(receive_finished(&mut runner).await.is_none());

        runner.spawn_effect(
            EffectId(1),
            TurnEpoch(1),
            EffectSpec::ModelCall(empty_window()),
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("model terminal");
        assert!(
            matches!(terminal, EffectTerminal::Model(outcome) if outcome.text == "root success")
        );

        runner.spawn_effect(
            EffectId(2),
            TurnEpoch(1),
            EffectSpec::ModelCall(empty_window()),
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("model failure");
        assert!(matches!(
            terminal,
            EffectTerminal::Failed { error: ErrorCode::ModelTransport, message }
                if message == "root failure"
        ));

        runner.spawn_effect(
            EffectId(3),
            TurnEpoch(2),
            EffectSpec::Tool(ProposedToolCall {
                name: String::from("read"),
                args_json: String::from("{\"path\":\"tool.txt\"}"),
            }),
        );
        let (id, epoch, terminal) = receive_finished(&mut runner).await.expect("tool terminal");
        assert_eq!((id, epoch), (EffectId(3), TurnEpoch(2)));
        assert!(matches!(
            terminal,
            EffectTerminal::Tool { outcome: ToolOutcome::Succeeded, output }
                if output == "tool output"
        ));

        runner.spawn_effect(
            EffectId(4),
            TurnEpoch(2),
            EffectSpec::StoreReadPage {
                sel: Sel::Whole,
                cursor: None,
            },
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("page terminal");
        assert!(matches!(terminal, EffectTerminal::Pages(page) if page.records.len() == 1));

        runner.spawn_effect(
            EffectId(5),
            TurnEpoch(3),
            EffectSpec::SubModel {
                requests: vec![
                    AskRequest {
                        index: 8,
                        question: String::from("one?"),
                        context: String::from("context"),
                        sample_k: None,
                    },
                    AskRequest {
                        index: 9,
                        question: String::from("two?"),
                        context: String::from("context"),
                        sample_k: Some(2),
                    },
                ],
            },
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("ask terminal");
        assert!(matches!(
            terminal,
            EffectTerminal::Ask(results)
                if results.len() == 2
                    && results[0].index == 8
                    && results[0].answer == "sub one"
                    && results[0].tokens == 7
                    && results[1].answer == "sub two"
        ));

        runner.spawn_effect(
            EffectId(6),
            TurnEpoch(3),
            EffectSpec::SubModel {
                requests: vec![AskRequest {
                    index: 10,
                    question: String::from("fail?"),
                    context: String::new(),
                    sample_k: None,
                }],
            },
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("ask failure");
        assert!(matches!(
            terminal,
            EffectTerminal::Failed { error: ErrorCode::ModelTransport, message }
                if message == "sub failure"
        ));

        std::fs::remove_file(runner.appender.path()).expect("unlink open log");
        runner.spawn_effect(
            EffectId(7),
            TurnEpoch(4),
            EffectSpec::StoreReadPage {
                sel: Sel::Whole,
                cursor: None,
            },
        );
        let (_, _, terminal) = receive_finished(&mut runner).await.expect("store failure");
        assert!(matches!(
            terminal,
            EffectTerminal::Failed {
                error: ErrorCode::StoreIo,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn start_effect_discharge_and_interrupt_are_driven_to_idle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut direct = open_runner(&dir, "direct-start.jsonl", jail(vec![success_step("done")]));
        direct.discharge(vec![CoreAction::StartEffect {
            id: EffectId(50),
            epoch: TurnEpoch(0),
            spec: EffectSpec::ModelCall(empty_window()),
        }]);
        assert_eq!(direct.outstanding, 1);
        direct.run_to_idle().await;
        assert_eq!(direct.outstanding, 0);

        let mut runner = open_runner(&dir, "interrupt.jsonl", jail(vec![success_step("late")]));
        runner.submit(submission(
            1,
            Op::UserInput {
                text: String::from("begin"),
            },
        ));
        runner.submit(submission(2, Op::Interrupt));
        let events = runner.run_to_idle().await;
        assert!(events.iter().any(|event| matches!(
            event,
            Event::TurnEnded {
                reason: TurnEnd::Interrupted,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn closed_completion_channels_exit_both_idle_wait_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut disconnected = open_runner(&dir, "disconnected.jsonl", jail(Vec::new()));
        let (dead_tx, dead_rx) = mpsc::unbounded_channel();
        drop(dead_rx);
        let old_tx = std::mem::replace(&mut disconnected.tx, dead_tx);
        drop(old_tx);
        assert!(disconnected.run_to_idle().await.is_empty());

        let mut waiting = open_runner(&dir, "recv-none.jsonl", jail(Vec::new()));
        waiting.outstanding = 1;
        let (dead_tx, dead_rx) = mpsc::unbounded_channel();
        drop(dead_rx);
        let old_tx = std::mem::replace(&mut waiting.tx, dead_tx);
        let dropper = tokio::spawn(async move {
            tokio::task::yield_now().await;
            drop(old_tx);
        });
        assert!(waiting.run_to_idle().await.is_empty());
        dropper.await.expect("dropper task");
    }

    #[test]
    fn paging_selection_and_ask_result_helpers_cover_boundaries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("paging.jsonl");
        let mut records = vec![header(4)];
        records.extend((1..=70).map(|seq| event_record(seq, seq % 3)));
        let text = records
            .iter()
            .map(|record| serde_json::to_string(record).expect("serialize"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&path, text).expect("seed paging log");

        let first = read_store_page(&path, &Sel::Whole, None).expect("first page");
        assert_eq!(first.records.len(), 64);
        assert_eq!(first.next_cursor, Some(64));
        let second = read_store_page(&path, &Sel::Whole, first.next_cursor).expect("second page");
        assert_eq!(second.records.len(), 7);
        assert_eq!(second.next_cursor, None);

        let seq_range = Sel::Range(Range {
            unit: RangeUnit::Seq,
            start: 5,
            end: 8,
        });
        let range_page = read_store_page(&path, &seq_range, Some(1)).expect("range cursor");
        assert_eq!(
            range_page
                .records
                .iter()
                .map(|record| record.seq)
                .collect::<Vec<_>>(),
            vec![6, 7]
        );

        let record = event_record(7, 2);
        assert!(record_selected(
            &Sel::Range(Range {
                unit: RangeUnit::Turn,
                start: 2,
                end: 3,
            }),
            &record,
            100,
        ));
        assert!(record_selected(
            &Sel::Range(Range {
                unit: RangeUnit::Byte,
                start: 90,
                end: 110,
            }),
            &record,
            100,
        ));
        assert!(!record_selected(
            &Sel::Ref(Ref::<Records>::new(1)),
            &record,
            0
        ));

        let with_usage = ask_result(
            3,
            ModelOutcome {
                text: String::from("answer"),
                tool_calls: Vec::new(),
                usage: Some(Usage {
                    prompt_tokens: 12,
                    prompt_bytes: 120,
                }),
            },
            45,
        );
        assert_eq!((with_usage.index, with_usage.tokens), (3, 12));
        let without_usage = ask_result(
            4,
            ModelOutcome {
                text: String::from("answer"),
                tool_calls: Vec::new(),
                usage: None,
            },
            0,
        );
        assert_eq!(without_usage.tokens, 0);

        std::fs::write(&path, "not-json\n").expect("seed invalid log");
        assert_eq!(
            read_store_page(&path, &Sel::Whole, None)
                .expect_err("invalid record")
                .kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_store_page(&dir.path().join("missing"), &Sel::Whole, None)
                .expect_err("missing log")
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }
}
