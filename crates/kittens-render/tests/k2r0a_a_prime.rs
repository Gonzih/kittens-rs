//! K2R-0A transfer-boundary trace oracles (exit-review round 1 model:
//! one shared, reusable done-slot — the analogue of the single static SPI2
//! ISR slot — with explicit active/disarm state, cancel-settlement
//! linearization, and drop disarming).

#![allow(missing_docs)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use kittens_render::geometry::{FrameEpoch, Region};
use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

fn epoch(n: u64) -> FrameEpoch {
    // FrameEpoch has no public constructor by design; tests obtain real
    // epochs through FrameDemand in the sweep suite. Here the transfer
    // boundary is tested in isolation with a demand-minted epoch.
    let plan = kittens_render::sweep::SweepPlan::new(REGION_FULL, 4).expect("plan");
    let mut demand = kittens_render::demand::FrameDemand::new(0, plan);
    let mut minted = None;
    for _ in 0..=n {
        demand.request();
        let sweep = demand
            .begin_sweep(kittens_render::demand::Tick(0), ())
            .expect("mint");
        minted = Some(sweep.epoch());
        let (aborted, ()) = sweep.abort();
        demand
            .finish_failed(aborted, kittens_render::demand::Tick(0))
            .expect("active");
    }
    minted.expect("minted at least one epoch")
}

const REGION_FULL: Region = Region {
    x: 0,
    y: 0,
    width: 8,
    height: 4,
};

struct CountingWaker {
    wakes: AtomicUsize,
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

fn counting_waker() -> (Arc<CountingWaker>, Waker) {
    let inner = Arc::new(CountingWaker {
        wakes: AtomicUsize::new(0),
    });
    let waker = Waker::from(Arc::clone(&inner));
    (inner, waker)
}

/// The single shared done-slot: the model analogue of the adapter's one
/// static ISR slot. Reused across sequential transfers; `active` mirrors
/// the arm/disarm discipline of the verdict's SPI2 design.
#[derive(Default)]
struct SharedSlot {
    state: Mutex<SlotState>,
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)] // test model mirrors the ISR slot flags
struct SlotState {
    active: bool,
    done: bool,
    fail: bool,
    complete_on_register: bool,
    waker: Option<Waker>,
}

impl SharedSlot {
    /// The model "interrupt": wakes only an active registration.
    fn complete(&self) {
        let waker = {
            let mut slot = self.state.lock().expect("slot lock");
            slot.done = true;
            if slot.active { slot.waker.take() } else { None }
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn fail(&self) {
        self.state.lock().expect("slot lock").fail = true;
        self.complete();
    }

    fn is_disarmed(&self) -> bool {
        let slot = self.state.lock().expect("slot lock");
        !slot.active && slot.waker.is_none()
    }
}

struct ModelTransport(&'static str);
struct ModelBuffer(&'static str);
struct ModelSpare(&'static str);

/// One model transfer over the shared slot. Arms the slot on start;
/// disarms on recovery *and* on drop (the reviewed adapter's Drop
/// obligation, finding 1).
struct ModelTransfer {
    slot: Arc<SharedSlot>,
    transport: Option<ModelTransport>,
    buffer: Option<ModelBuffer>,
    settled: Option<TransferOutcome>,
}

fn start_on(
    slot: &Arc<SharedSlot>,
    transport: ModelTransport,
    buffer: ModelBuffer,
) -> ModelTransfer {
    {
        let mut state = slot.state.lock().expect("slot lock");
        state.active = true;
        state.done = false;
        state.fail = false;
        state.waker = None;
    }
    ModelTransfer {
        slot: Arc::clone(slot),
        transport: Some(transport),
        buffer: Some(buffer),
        settled: None,
    }
}

impl ModelTransfer {
    fn disarm_slot(&self) {
        let mut state = self.slot.state.lock().expect("slot lock");
        state.active = false;
        state.waker = None;
    }
}

impl OwnedTransfer for ModelTransfer {
    type Transport = ModelTransport;
    type Buffer = ModelBuffer;

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_some() {
            return Poll::Ready(());
        }
        let outcome = {
            let mut state = self.slot.state.lock().expect("slot lock");
            // Fast path.
            if state.done {
                Some(if state.fail {
                    TransferOutcome::Failed
                } else {
                    TransferOutcome::Completed
                })
            } else {
                // Register-then-recheck under the same exclusion the model
                // "ISR" uses; deterministic race injection lands between.
                state.waker = Some(cx.waker().clone());
                if state.complete_on_register {
                    state.complete_on_register = false;
                    state.done = true;
                }
                if state.done {
                    state.waker = None;
                    Some(if state.fail {
                        TransferOutcome::Failed
                    } else {
                        TransferOutcome::Completed
                    })
                } else {
                    None
                }
            }
        };
        match outcome {
            Some(outcome) => {
                self.settled = Some(outcome);
                Poll::Ready(())
            }
            None => Poll::Pending,
        }
    }

    fn cancel(&mut self) {
        if self.settled.is_some() {
            return;
        }
        // The cancellation observation is the linearization point (finding
        // 2): classify and STORE the settlement here, atomically with the
        // completion observation. A hardware completion landing after this
        // point is conservatively Cancelled and cannot rewrite the outcome.
        let (outcome, waker) = {
            let mut state = self.slot.state.lock().expect("slot lock");
            let outcome = if state.done {
                if state.fail {
                    TransferOutcome::Failed
                } else {
                    TransferOutcome::Completed
                }
            } else {
                TransferOutcome::Cancelled
            };
            state.active = false;
            (outcome, state.waker.take())
        };
        self.settled = Some(outcome);
        // Cancellation is progress and may produce no hardware interrupt.
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn recover(mut self) -> Recovered<ModelTransport, ModelBuffer> {
        let outcome = self.settled.expect("recover called before settlement");
        self.disarm_slot();
        Recovered {
            transport: self.transport.take().expect("transport held"),
            buffer: self.buffer.take().expect("buffer held"),
            outcome,
        }
    }
}

impl Drop for ModelTransfer {
    fn drop(&mut self) {
        // A dropped pending transfer must not leave a stale registration:
        // the adapter's Drop disarms the slot (finding 1's drop trace).
        if self.transport.is_some() {
            self.disarm_slot();
        }
    }
}

fn flight_on(slot: &Arc<SharedSlot>, e: u64) -> InFlight<ModelTransfer, ModelSpare> {
    InFlight::new(
        start_on(slot, ModelTransport("qspi"), ModelBuffer("buf-a")),
        ModelSpare("spare-0"),
        epoch(e),
        REGION_FULL,
    )
}

#[test]
fn polled_then_lost_arbitration_gets_exactly_one_wake() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0, "no self-wake");

    slot.complete();
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        1,
        "one completion wake"
    );

    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Completed);
            assert_eq!(settled.transport.0, "qspi");
            assert_eq!(settled.spare.0, "spare-0");
            assert!(
                settled.stripe_written().is_some(),
                "completed mints a witness"
            );
        }
        Poll::Pending => panic!("completed transfer must recover"),
    }
    assert!(flight.is_spent());
    assert!(slot.is_disarmed(), "recovery disarms the shared slot");
}

#[test]
fn unpolled_below_winner_recovers_on_first_poll() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    slot.complete();

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_ready());
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn completion_during_registration_is_not_lost() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    slot.state.lock().expect("slot lock").complete_on_register = true;

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(
        flight.poll_complete(&mut cx).is_ready(),
        "register-then-recheck observes the racing completion in the same poll"
    );
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0);
}

/// Finding 2's adversarial oracle: cancel first, hardware completes late,
/// the settlement stays Cancelled — the cancellation observation was the
/// linearization point.
#[test]
fn cancel_then_late_completion_stays_cancelled() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    flight.begin_drain();
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 1, "cancel wakes");

    // Hardware completes AFTER the cancellation linearization point,
    // before the repoll.
    slot.complete();

    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(
                settled.outcome,
                TransferOutcome::Cancelled,
                "late completion cannot rewrite a cancelled settlement"
            );
            assert!(
                settled.stripe_written().is_none(),
                "a cancelled stripe mints no coverage witness"
            );
        }
        Poll::Pending => panic!("a drained transfer must settle"),
    }
}

#[test]
fn drain_racing_prior_completion_reports_completed() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    slot.complete();
    flight.begin_drain();

    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => assert_eq!(settled.outcome, TransferOutcome::Completed),
        Poll::Pending => panic!("settled transfer must recover"),
    }
}

#[test]
fn failure_settles_returns_resources_and_mints_no_witness() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_pending());

    slot.fail();
    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Failed);
            assert_eq!(settled.buffer.0, "buf-a", "buffer recovered");
            assert!(settled.stripe_written().is_none());
        }
        Poll::Pending => panic!("failed transfer must settle"),
    }
}

#[test]
fn waker_replacement_wakes_only_the_newest() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (old_counter, old_waker) = counting_waker();
    let (new_counter, new_waker) = counting_waker();

    assert!(
        flight
            .poll_complete(&mut Context::from_waker(&old_waker))
            .is_pending()
    );
    assert!(
        flight
            .poll_complete(&mut Context::from_waker(&new_waker))
            .is_pending()
    );

    slot.complete();
    assert_eq!(
        old_counter.wakes.load(Ordering::SeqCst),
        0,
        "stale waker silent"
    );
    assert_eq!(new_counter.wakes.load(Ordering::SeqCst), 1, "newest woken");
}

/// Finding 1's late-IRQ trace, now meaningful: recovery disarmed the
/// SHARED slot, so a late "interrupt" wakes nobody even though the slot
/// object persists.
#[test]
fn late_completion_after_recovery_is_inert_via_disarm() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    slot.complete();
    assert!(flight.poll_complete(&mut cx).is_ready());
    let wakes_at_recovery = counter.wakes.load(Ordering::SeqCst);
    assert!(slot.is_disarmed());

    slot.complete(); // late spurious interrupt on the same shared slot
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        wakes_at_recovery,
        "wakes nobody"
    );
}

/// Finding 1's drop trace: dropping a pending in-flight transfer disarms
/// the shared slot; a late interrupt wakes nobody. Resource recovery is
/// intentionally lost on this path (the documented non-returning
/// boundary) — but no stale registration survives.
#[test]
fn dropped_pending_transfer_disarms_the_slot() {
    let slot = Arc::new(SharedSlot::default());
    let (counter, waker) = counting_waker();
    {
        let mut flight = flight_on(&slot, 0);
        let mut cx = Context::from_waker(&waker);
        assert!(flight.poll_complete(&mut cx).is_pending());
        // flight dropped here with the transfer pending.
    }
    assert!(slot.is_disarmed(), "drop disarms");
    slot.complete();
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0, "no stale wake");
}

/// Finding 1's N→N+1 trace against the SAME shared slot: the second
/// transfer re-arms the slot the first one used and completes cleanly.
#[test]
fn sequential_transfers_reuse_the_same_slot() {
    let slot = Arc::new(SharedSlot::default());
    let (_c, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    let mut first = flight_on(&slot, 0);
    slot.complete();
    let settled = match first.poll_complete(&mut cx) {
        Poll::Ready(s) => s,
        Poll::Pending => panic!("first settles"),
    };
    assert!(slot.is_disarmed());

    // Second transfer on the SAME slot with the recovered transport.
    let second_transfer = start_on(&slot, settled.transport, ModelBuffer("buf-b"));
    let mut second = InFlight::new(second_transfer, settled.spare, epoch(1), REGION_FULL);
    assert!(second.poll_complete(&mut cx).is_pending());
    slot.complete();
    match second.poll_complete(&mut cx) {
        Poll::Ready(s) => {
            assert_eq!(s.outcome, TransferOutcome::Completed);
            assert_eq!(s.transport.0, "qspi", "same transport identity");
            assert_eq!(s.spare.0, "spare-0", "same spare identity");
        }
        Poll::Pending => panic!("second settles"),
    }
}

#[test]
fn spare_is_writable_during_flight_and_drain_flag_clears() {
    let slot = Arc::new(SharedSlot::default());
    let mut flight = flight_on(&slot, 0);
    let (_c, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert!(flight.spare_mut().is_some());

    flight.begin_drain();
    assert!(flight.is_draining());
    assert!(flight.poll_complete(&mut cx).is_ready());
    assert!(
        !flight.is_draining(),
        "settled adapter is no longer draining"
    );
}

// ---------------------------------------------------------------------------
// Negative control: check-then-register loses the racing completion. Kept
// from round 1; proves the adversarial oracle is load-bearing.
// ---------------------------------------------------------------------------

struct BrokenTransfer {
    slot: Arc<SharedSlot>,
    settled: bool,
}

impl BrokenTransfer {
    fn poll_done_check_then_register(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.settled {
            return Poll::Ready(());
        }
        let mut state = self.slot.state.lock().expect("slot lock");
        // BROKEN ORDER: check first...
        if state.done {
            drop(state);
            self.settled = true;
            return Poll::Ready(());
        }
        // ...completion lands here...
        if state.complete_on_register {
            state.complete_on_register = false;
            state.done = true;
            // fires with no waker registered: wakes nobody.
        }
        // ...then register, too late, with no recheck.
        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[test]
fn negative_control_check_then_register_loses_the_wake() {
    let slot = Arc::new(SharedSlot::default());
    {
        let mut state = slot.state.lock().expect("slot lock");
        state.active = true;
        state.complete_on_register = true;
    }
    let mut broken = BrokenTransfer {
        slot: Arc::clone(&slot),
        settled: false,
    };

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(broken.poll_done_check_then_register(&mut cx).is_pending());
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0, "the wake is lost");
    assert!(
        slot.state.lock().expect("slot lock").done,
        "hardware really completed — the event exists and was lost"
    );
}
