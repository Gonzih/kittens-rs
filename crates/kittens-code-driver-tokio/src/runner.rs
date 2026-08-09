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
use std::{fs::File, io::BufRead, io::BufReader, path::Path};

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine, ResumeError,
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
                        match model.complete_submodel(request).await {
                            Ok(outcome) => results.push(AskResult {
                                index,
                                answer: outcome.text,
                            }),
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
