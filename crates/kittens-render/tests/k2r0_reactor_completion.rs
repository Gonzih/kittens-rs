#![allow(clippy::ignored_unit_patterns, missing_docs)]

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use kittens::reactor::Control;
use kittens::source::{Latched, OptionalInlineOneShot};
use kittens_render::demand::{FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::{AbortedSweep, Sweep, SweepPlan, SweepWritten};
use kittens_render::transfer::{
    FlightStarter, InFlight, OwnedTransfer, Recovered, StartPermit, TransferOutcome,
};

const PANEL: Region = Region {
    x: 0,
    y: 0,
    width: 4,
    height: 4,
};

#[derive(Debug)]
struct Transport(u8);

#[derive(Debug)]
struct Buffer(u8);

#[derive(Debug, Default)]
struct CompletionSlot {
    outcome: Mutex<Option<TransferOutcome>>,
    waker: Mutex<Option<Waker>>,
    polls: AtomicUsize,
    wakes: AtomicUsize,
    cancels: AtomicUsize,
    drop_disarms: AtomicUsize,
    disarmed: AtomicBool,
}

impl CompletionSlot {
    fn poll_done(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.polls.fetch_add(1, Ordering::SeqCst);

        // The model uses the production contract's register-then-recheck
        // order so a completion racing registration remains level-visible.
        *self.waker.lock().expect("completion waker") = Some(cx.waker().clone());
        if self.outcome.lock().expect("completion outcome").is_some() {
            self.waker.lock().expect("completion waker").take();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn complete(&self) {
        self.settle(TransferOutcome::Completed);
    }

    fn cancel(&self) {
        self.cancels.fetch_add(1, Ordering::SeqCst);
        self.settle(TransferOutcome::Cancelled);
    }

    fn settle(&self, outcome: TransferOutcome) {
        let installed = {
            let mut current = self.outcome.lock().expect("completion outcome");
            if current.is_some() {
                false
            } else {
                *current = Some(outcome);
                true
            }
        };
        if installed {
            let waker = self.waker.lock().expect("completion waker").take();
            if let Some(waker) = waker {
                self.wakes.fetch_add(1, Ordering::SeqCst);
                waker.wake();
            }
        }
    }

    fn outcome(&self) -> Option<TransferOutcome> {
        *self.outcome.lock().expect("completion outcome")
    }

    fn disarm_after_recovery(&self) {
        self.waker.lock().expect("completion waker").take();
        self.disarmed.store(true, Ordering::SeqCst);
    }

    fn cancel_and_disarm_on_drop(&self) {
        self.outcome
            .lock()
            .expect("completion outcome")
            .get_or_insert(TransferOutcome::Cancelled);
        self.waker.lock().expect("completion waker").take();
        self.disarmed.store(true, Ordering::SeqCst);
        self.drop_disarms.fetch_add(1, Ordering::SeqCst);
    }

    fn polls(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }

    fn wakes(&self) -> usize {
        self.wakes.load(Ordering::SeqCst)
    }

    fn cancels(&self) -> usize {
        self.cancels.load(Ordering::SeqCst)
    }

    fn drop_disarms(&self) -> usize {
        self.drop_disarms.load(Ordering::SeqCst)
    }

    fn is_disarmed(&self) -> bool {
        self.disarmed.load(Ordering::SeqCst)
    }

    fn has_registered_waker(&self) -> bool {
        self.waker.lock().expect("completion waker").is_some()
    }
}

#[derive(Debug)]
struct ModelTransfer {
    transport: Option<Transport>,
    buffer: Option<Buffer>,
    slot: Arc<CompletionSlot>,
    live: bool,
}

impl OwnedTransfer for ModelTransfer {
    type Transport = Transport;
    type Buffer = Buffer;

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.slot.poll_done(cx)
    }

    fn cancel(&mut self) {
        self.slot.cancel();
    }

    fn recover(mut self) -> Recovered<Self::Transport, Self::Buffer> {
        let outcome = self
            .slot
            .outcome()
            .expect("recovery follows ready settlement");
        self.slot.disarm_after_recovery();
        self.live = false;
        Recovered {
            transport: self.transport.take().expect("live transport"),
            buffer: self.buffer.take().expect("live sent buffer"),
            outcome,
        }
    }
}

impl Drop for ModelTransfer {
    fn drop(&mut self) {
        if self.live {
            // This is the reviewed adapter contract modeled by the test: a
            // dropped live transfer synchronously cancels and disarms its
            // registered completion before returning.
            self.slot.cancel_and_disarm_on_drop();
            self.live = false;
        }
    }
}

struct ModelStarter {
    transport: Transport,
    buffer: Buffer,
    slot: Arc<CompletionSlot>,
    expected_region: Region,
}

impl FlightStarter for ModelStarter {
    type Transfer = ModelTransfer;
    type Error = Infallible;

    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        assert_eq!(region, self.expected_region);
        Ok(ModelTransfer {
            transport: Some(self.transport),
            buffer: Some(self.buffer),
            slot: self.slot,
            live: true,
        })
    }
}

type Flight = InFlight<ModelTransfer, Buffer>;
type CompletionSource = OptionalInlineOneShot<Flight>;

fn demand_and_sweep(stripe_height: u16) -> (FrameDemand, Sweep<()>) {
    let geometry = PanelGeometry::custom_unvalidated_panel(PANEL);
    let plan = SweepPlan::for_panel(geometry, stripe_height).expect("valid test plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let sweep = demand
        .begin_sweep(Tick(0), ())
        .expect("requested sweep is immediately eligible");
    (demand, sweep)
}

fn start_next(
    sweep: &mut Sweep<()>,
    transport: Transport,
    sent: Buffer,
    spare: Buffer,
    slot: Arc<CompletionSlot>,
) -> Flight {
    let target = sweep.next_target().expect("another planned stripe");
    let expected_region = target.region();
    target
        .start_flight(
            spare,
            ModelStarter {
                transport,
                buffer: sent,
                slot,
                expected_region,
            },
        )
        .expect("model start cannot reject")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceEvent {
    Trigger(usize),
    Completion(usize),
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // One trace must retain both stripes and the shared carrier.
async fn completion_survives_poll_then_loss_and_same_carrier_rearms_for_next_stripe() {
    struct Sources {
        transfer_done: CompletionSource,
        trigger: Latched<()>,
    }

    let slots = [
        Arc::new(CompletionSlot::default()),
        Arc::new(CompletionSlot::default()),
    ];
    let (mut demand, mut first_sweep) = demand_and_sweep(2);
    let first_flight = start_next(
        &mut first_sweep,
        Transport(7),
        Buffer(0),
        Buffer(1),
        Arc::clone(&slots[0]),
    );
    let mut sweep = Some(first_sweep);
    let mut sources = Sources {
        transfer_done: CompletionSource::from_future(first_flight),
        trigger: Latched::new(),
    };
    sources.trigger.arm(()).expect("initial local arm");

    let mut trigger_index = 0;
    let mut completion_index = 0;
    let mut polls_before_trigger = Vec::new();
    let mut recoveries = Vec::new();
    let mut trace = Vec::new();

    let result: Result<SweepWritten, Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        /// Completion leads so each armed flight is polled pending before the
        /// local trigger wins; the carrier must retain that exact flight.
        #[source(transfer_done)]
        #[readiness(quiescent)]
        settled = sources.transfer_done => {
            trace.push(TraceEvent::Completion(completion_index));
            assert_eq!(settled.outcome(), TransferOutcome::Completed);
            let region = settled.region();
            let (transport, sent, spare, settlement) = settled.into_parts();
            recoveries.push((transport.0, sent.0, spare.0, region));
            assert_eq!(
                sweep
                    .as_mut()
                    .expect("active sweep")
                    .settle(settlement),
                Ok(TransferOutcome::Completed)
            );
            completion_index += 1;

            if completion_index == 1 {
                let second_flight = start_next(
                    sweep.as_mut().expect("active sweep"),
                    transport,
                    spare,
                    sent,
                    Arc::clone(&slots[1]),
                );
                sources
                    .transfer_done
                    .arm(second_flight)
                    .expect("carrier is dormant before its handler runs");
                sources
                    .trigger
                    .arm(())
                    .expect("second local trigger arm");
                Ok(Control::Continue)
            } else {
                let (written, ()) = sweep
                    .take()
                    .expect("active sweep")
                    .finish()
                    .expect("both stripes settled written");
                Ok(Control::Stop(written))
            }
        }

        /// The local trigger models an interrupt landing only after the
        /// completion arm has registered the reactor waker and lost selection.
        #[source(trigger)]
        #[readiness(quiescent)]
        _ = sources.trigger => {
            polls_before_trigger.push(slots[trigger_index].polls());
            trace.push(TraceEvent::Trigger(trigger_index));
            slots[trigger_index].complete();
            trigger_index += 1;
            Ok(Control::Continue)
        }
    };

    let written = result.expect("reactor completes the sweep");
    assert_eq!(
        demand.finish_written(written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    assert_eq!(polls_before_trigger, [1, 1]);
    assert_eq!(slots[0].wakes(), 1);
    assert_eq!(slots[1].wakes(), 1);
    assert_eq!(
        recoveries,
        [
            (
                7,
                0,
                1,
                Region {
                    x: 0,
                    y: 0,
                    width: 4,
                    height: 2
                }
            ),
            (
                7,
                1,
                0,
                Region {
                    x: 0,
                    y: 2,
                    width: 4,
                    height: 2
                }
            ),
        ]
    );
    assert_eq!(
        trace,
        [
            TraceEvent::Trigger(0),
            TraceEvent::Completion(0),
            TraceEvent::Trigger(1),
            TraceEvent::Completion(1),
        ]
    );
    assert!(sources.transfer_done.is_dormant());
    assert!(sources.transfer_done.future_mut().is_none());
}

#[tokio::test]
async fn completion_survives_earlier_winner_before_its_first_poll() {
    struct Sources {
        earlier_winner: Latched<()>,
        transfer_done: CompletionSource,
    }

    let slot = Arc::new(CompletionSlot::default());
    let (mut demand, mut active_sweep) = demand_and_sweep(4);
    let flight = start_next(
        &mut active_sweep,
        Transport(8),
        Buffer(2),
        Buffer(3),
        Arc::clone(&slot),
    );
    let mut sweep = Some(active_sweep);
    let mut sources = Sources {
        earlier_winner: Latched::new(),
        transfer_done: CompletionSource::from_future(flight),
    };
    sources.earlier_winner.arm(()).expect("initial local arm");
    let mut polls_before_trigger = None;
    let mut trace = Vec::new();

    let result: Result<SweepWritten, Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        /// This earlier source wins the first arbitration before the completion
        /// source can be polled; local completion must remain observable later.
        #[source(earlier_winner)]
        #[readiness(quiescent)]
        _ = sources.earlier_winner => {
            polls_before_trigger = Some(slot.polls());
            trace.push(TraceEvent::Trigger(0));
            slot.complete();
            Ok(Control::Continue)
        }

        /// The retained completion first sees the transfer only in the next
        /// arbitration, after the earlier handler made it ready without a wake.
        #[source(transfer_done)]
        #[readiness(quiescent)]
        settled = sources.transfer_done => {
            trace.push(TraceEvent::Completion(0));
            let (transport, sent, spare, settlement) = settled.into_parts();
            assert_eq!((transport.0, sent.0, spare.0), (8, 2, 3));
            assert_eq!(
                sweep
                    .as_mut()
                    .expect("active sweep")
                    .settle(settlement),
                Ok(TransferOutcome::Completed)
            );
            let (written, ()) = sweep
                .take()
                .expect("active sweep")
                .finish()
                .expect("single stripe settled written");
            Ok(Control::Stop(written))
        }
    };

    let written = result.expect("reactor completes the sweep");
    assert_eq!(
        demand.finish_written(written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    assert_eq!(polls_before_trigger, Some(0));
    assert_eq!(slot.polls(), 1);
    assert_eq!(slot.wakes(), 0);
    assert_eq!(trace, [TraceEvent::Trigger(0), TraceEvent::Completion(0)]);
    assert!(sources.transfer_done.is_dormant());
}

#[tokio::test]
async fn graceful_drain_uses_future_mut_and_reconciles_cancelled_settlement() {
    struct Sources {
        transfer_done: CompletionSource,
        drain_request: Latched<()>,
    }

    let slot = Arc::new(CompletionSlot::default());
    let (mut demand, mut active_sweep) = demand_and_sweep(4);
    let flight = start_next(
        &mut active_sweep,
        Transport(9),
        Buffer(4),
        Buffer(5),
        Arc::clone(&slot),
    );
    let mut sweep = Some(active_sweep);
    let mut sources = Sources {
        transfer_done: CompletionSource::from_future(flight),
        drain_request: Latched::new(),
    };
    sources.drain_request.arm(()).expect("initial local arm");

    let result: Result<AbortedSweep, Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        /// Completion leads so the cancellation path proves a registered
        /// reactor waker is signaled and then continues to owned recovery.
        #[source(transfer_done)]
        #[readiness(quiescent)]
        settled = sources.transfer_done => {
            assert_eq!(settled.outcome(), TransferOutcome::Cancelled);
            let (transport, sent, spare, settlement) = settled.into_parts();
            assert_eq!((transport.0, sent.0, spare.0), (9, 4, 5));
            assert_eq!(
                sweep
                    .as_mut()
                    .expect("active sweep")
                    .settle(settlement),
                Ok(TransferOutcome::Cancelled)
            );
            assert!(sweep.as_ref().expect("active sweep").is_poisoned());
            let (aborted, ()) = sweep
                .take()
                .expect("active sweep")
                .abort()
                .expect("cancelled settlement clears outstanding target");
            Ok(Control::Stop(aborted))
        }

        /// Graceful shutdown borrows the installed flight only to request its
        /// canonical drain transition, then leaves arbitration running.
        #[source(drain_request)]
        #[readiness(quiescent)]
        _ = sources.drain_request => {
            assert_eq!(slot.polls(), 1);
            let flight = sources
                .transfer_done
                .future_mut()
                .expect("armed flight to drain");
            assert!(!flight.is_draining());
            flight.begin_drain();
            assert!(flight.is_draining());
            Ok(Control::Continue)
        }
    };

    let aborted = result.expect("reactor drains the cancelled flight");
    assert_eq!(demand.finish_failed(aborted, Tick(1)), Ok(()));
    assert_eq!(slot.cancels(), 1);
    assert_eq!(slot.wakes(), 1);
    assert_eq!(slot.drop_disarms(), 0);
    assert!(slot.is_disarmed());
    assert!(sources.transfer_done.is_dormant());
    assert!(sources.transfer_done.future_mut().is_none());
}

#[tokio::test]
async fn dropping_carrier_after_reactor_exit_invokes_transfer_drop_disarm() {
    struct Sources {
        transfer_done: CompletionSource,
        exit: Latched<()>,
    }

    let slot = Arc::new(CompletionSlot::default());
    let (_demand, mut sweep) = demand_and_sweep(4);
    let flight = start_next(
        &mut sweep,
        Transport(10),
        Buffer(6),
        Buffer(7),
        Arc::clone(&slot),
    );
    let mut sources = Sources {
        transfer_done: CompletionSource::from_future(flight),
        exit: Latched::new(),
    };
    sources.exit.arm(()).expect("initial local arm");
    let mut completion_selected = false;

    let result: Result<(), Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        /// Poll the live flight before the terminal source wins so the model
        /// has a real registered completion slot when the carrier is dropped.
        #[source(transfer_done)]
        #[readiness(quiescent)]
        settled = sources.transfer_done => {
            completion_selected = true;
            drop(settled);
            Ok(Control::Stop(()))
        }

        /// This models a non-draining raw reactor exit: resources do not return,
        /// but dropping the reviewed transfer must synchronously disarm it.
        #[source(exit)]
        #[readiness(quiescent)]
        #[terminal]
        _ = sources.exit => {
            Ok(())
        }
    };

    assert_eq!(result, Ok(()));
    assert!(!completion_selected);
    assert_eq!(slot.polls(), 1);
    assert!(slot.has_registered_waker());
    assert_eq!(slot.drop_disarms(), 0);
    assert!(!sources.transfer_done.is_dormant());

    drop(sources);

    assert_eq!(slot.drop_disarms(), 1);
    assert!(slot.is_disarmed());
    assert!(!slot.has_registered_waker());
    assert_eq!(slot.outcome(), Some(TransferOutcome::Cancelled));
}
