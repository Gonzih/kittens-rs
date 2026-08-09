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

use kittens_code_core::engine::{CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine};
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{EffectId, TurnEpoch};
use kittens_code_protocol::op::Submission;
use tokio::sync::mpsc;

use crate::appender::Appender;
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
        }
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
            self.discharge(transition.actions).await;
        }
        &self.published
    }

    /// Discharges one bounded action batch as a unit (L-A2b whole-batch
    /// dispatch), spawning owned effect tasks and bumping the outstanding
    /// counter.
    async fn discharge(&mut self, actions: Vec<CoreAction>) {
        for action in actions {
            match action {
                CoreAction::Commit(records) => {
                    match self.appender.append(&records) {
                        Ok(watermark) => {
                            let t = self.engine.handle(CoreInput::Persisted {
                                up_to_seq: watermark,
                            });
                            // Persisted handling produces no further actions
                            // in KC0 (watermark bookkeeping only).
                            debug_assert!(t.actions.is_empty());
                        }
                        Err((at_seq, e)) => {
                            let t = self.engine.handle(CoreInput::PersistFailed {
                                at_seq,
                                message: e.to_string(),
                            });
                            // Fatal path: publish and cancel actions follow.
                            Box::pin(self.discharge(t.actions)).await;
                        }
                    }
                }
                CoreAction::Publish(event) => {
                    // L-A3: only publish once the covering records are
                    // durable. In KC0 commits are synchronous and already
                    // acknowledged above, so the watermark is current.
                    self.published.push(event);
                }
                CoreAction::StartEffect { id, epoch, spec } => {
                    self.outstanding += 1;
                    self.spawn_effect(id, epoch, spec);
                }
                CoreAction::CancelEffect { .. } => {
                    // KC0 effects are short-lived owned tasks; cancellation
                    // is cooperative — a late terminal is dropped by the
                    // engine's exactly-once ledger. The streaming model
                    // client (post-KC0) wires real abort handles here.
                }
                // Unknown future core actions: a driver that cannot honor
                // one must refuse loudly rather than silently skip it.
                other => unimplemented!("driver missing CoreAction handler: {other:?}"),
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
            // Unknown future effect specs (l2 embed, store pages): the KC0
            // driver does not enable those features, so reaching here is a
            // configuration bug, not a runtime condition.
            other => unimplemented!("driver missing EffectSpec handler: {other:?}"),
        }
    }
}
