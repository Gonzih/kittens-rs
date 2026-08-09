//! K2R-0A trace oracles over the host model (SPEC section 7 pass criteria,
//! reviewer corrections applied).
//!
//! The model mirrors the verdict's SPI2 mechanism: an externally driven
//! completion ("interrupt"), a waker slot registered under the same
//! exclusion the ISR uses, register-then-recheck in `poll_done`, and a
//! cancel that wakes. A deliberately broken check-then-register model at
//! the bottom proves the adversarial oracle catches the lost-wake race.

#![allow(missing_docs)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

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

/// Model "hardware": completion driven externally like a transfer-done
/// interrupt. `complete_on_register` deterministically reproduces the race
/// where completion lands between waker registration and the recheck.
#[derive(Default)]
struct ModelHw {
    done: AtomicBool,
    fail: AtomicBool,
    cancelled: AtomicBool,
    complete_on_register: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl ModelHw {
    fn complete(&self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(waker) = self.waker.lock().expect("waker lock").take() {
            waker.wake();
        }
    }
    fn fail(&self) {
        self.fail.store(true, Ordering::SeqCst);
        self.complete();
    }
}

struct ModelTransport(&'static str);
struct ModelBuffer(Vec<u8>);
struct ModelSpare(&'static str);

struct ModelTransfer {
    hw: Arc<ModelHw>,
    transport: Option<ModelTransport>,
    buffer: Option<ModelBuffer>,
    settled: Option<TransferOutcome>,
}

fn start_model_transfer(
    transport: ModelTransport,
    buffer: ModelBuffer,
) -> (ModelTransfer, Arc<ModelHw>) {
    let hw = Arc::new(ModelHw::default());
    (
        ModelTransfer {
            hw: Arc::clone(&hw),
            transport: Some(transport),
            buffer: Some(buffer),
            settled: None,
        },
        hw,
    )
}

impl ModelTransfer {
    fn outcome_from_hw(&self) -> TransferOutcome {
        if self.hw.fail.load(Ordering::SeqCst) {
            TransferOutcome::Failed
        } else {
            TransferOutcome::Completed
        }
    }
}

impl OwnedTransfer for ModelTransfer {
    type Transport = ModelTransport;
    type Buffer = ModelBuffer;

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_some() {
            return Poll::Ready(());
        }
        if self.hw.cancelled.load(Ordering::SeqCst) && !self.hw.done.load(Ordering::SeqCst) {
            self.settled = Some(TransferOutcome::Cancelled);
            return Poll::Ready(());
        }
        // Register-then-recheck, per the mandated order. First a fast path:
        if self.hw.done.load(Ordering::SeqCst) {
            self.settled = Some(self.outcome_from_hw());
            return Poll::Ready(());
        }
        // Register.
        *self.hw.waker.lock().expect("waker lock") = Some(cx.waker().clone());
        // Deterministic race injection: completion lands "during"
        // registration, before the recheck.
        if self.hw.complete_on_register.swap(false, Ordering::SeqCst) {
            self.hw.done.store(true, Ordering::SeqCst);
        }
        // Recheck closes the completion-during-registration window.
        if self.hw.done.load(Ordering::SeqCst) {
            self.hw.waker.lock().expect("waker lock").take();
            self.settled = Some(self.outcome_from_hw());
            return Poll::Ready(());
        }
        Poll::Pending
    }

    fn cancel(&mut self) {
        if self.settled.is_some() {
            return;
        }
        self.hw.cancelled.store(true, Ordering::SeqCst);
        // Cancellation is progress and may produce no hardware interrupt:
        // wake the registered waker ourselves (reviewer correction 2).
        if let Some(waker) = self.hw.waker.lock().expect("waker lock").take() {
            waker.wake();
        }
    }

    fn recover(self) -> Recovered<ModelTransport, ModelBuffer> {
        let outcome = self.settled.expect("recover called before settlement");
        Recovered {
            transport: self.transport.expect("transport held until recovery"),
            buffer: self.buffer.expect("buffer held until recovery"),
            outcome,
        }
    }
}

fn in_flight() -> (InFlight<ModelTransfer, ModelSpare>, Arc<ModelHw>) {
    let (transfer, hw) = start_model_transfer(ModelTransport("qspi"), ModelBuffer(vec![0xAB; 16]));
    (InFlight::new(transfer, ModelSpare("spare-0")), hw)
}

#[test]
fn polled_then_lost_arbitration_gets_exactly_one_wake() {
    let (mut flight, hw) = in_flight();
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0, "no self-wake");

    hw.complete();
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        1,
        "one completion wake"
    );

    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Completed);
            assert_eq!(settled.transport.0, "qspi");
            assert_eq!(settled.buffer.0.len(), 16);
            assert_eq!(settled.spare.0, "spare-0", "spare returned at settlement");
        }
        Poll::Pending => panic!("completed transfer must recover"),
    }
    assert!(flight.is_spent());
}

#[test]
fn unpolled_below_winner_recovers_on_first_poll() {
    let (mut flight, hw) = in_flight();
    hw.complete();

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_ready());
    assert_eq!(counter.wakes.load(Ordering::SeqCst), 0);
}

/// The adversarial oracle from reviewer correction 1: completion lands
/// between waker registration and the recheck. Register-then-recheck makes
/// the same poll observe it; nothing is lost.
#[test]
fn completion_during_registration_is_not_lost() {
    let (mut flight, hw) = in_flight();
    hw.complete_on_register.store(true, Ordering::SeqCst);

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => assert_eq!(settled.outcome, TransferOutcome::Completed),
        Poll::Pending => panic!("register-then-recheck must observe the racing completion"),
    }
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        0,
        "settlement in the same poll needs no wake"
    );
}

/// Reviewer correction 2's required trace: pending poll → cancel → exactly
/// one progress wake → repoll recovers everything as Cancelled.
#[test]
fn cancel_wakes_the_pending_poller() {
    let (mut flight, hw) = in_flight();
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    flight.begin_drain();
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        1,
        "cancellation is progress and must wake"
    );

    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Cancelled);
            assert_eq!(settled.transport.0, "qspi");
            assert_eq!(settled.buffer.0.len(), 16);
            assert_eq!(settled.spare.0, "spare-0");
        }
        Poll::Pending => panic!("a drained transfer must settle"),
    }
    let _ = hw;
}

#[test]
fn drain_racing_completion_reports_completed() {
    let (mut flight, hw) = in_flight();
    hw.complete();
    flight.begin_drain();

    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => assert_eq!(settled.outcome, TransferOutcome::Completed),
        Poll::Pending => panic!("settled transfer must recover"),
    }
}

#[test]
fn failure_settles_and_returns_resources() {
    let (mut flight, hw) = in_flight();
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_pending());

    hw.fail();
    match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Failed);
            assert_eq!(settled.buffer.0.len(), 16);
        }
        Poll::Pending => panic!("failed transfer must settle"),
    }
}

/// Reviewer correction 8: waker replacement — a later poll with a different
/// waker replaces the registration; completion wakes only the newest.
#[test]
fn waker_replacement_wakes_only_the_newest() {
    let (mut flight, hw) = in_flight();
    let (old_counter, old_waker) = counting_waker();
    let (new_counter, new_waker) = counting_waker();

    let mut old_cx = Context::from_waker(&old_waker);
    let mut new_cx = Context::from_waker(&new_waker);

    assert!(flight.poll_complete(&mut old_cx).is_pending());
    assert!(flight.poll_complete(&mut new_cx).is_pending());

    hw.complete();
    assert_eq!(
        old_counter.wakes.load(Ordering::SeqCst),
        0,
        "stale waker silent"
    );
    assert_eq!(
        new_counter.wakes.load(Ordering::SeqCst),
        1,
        "newest waker woken"
    );
}

/// Reviewer correction 8: a late "interrupt" after recovery must not wake
/// anything — the slot was cleared at settlement.
#[test]
fn late_completion_after_recovery_is_inert() {
    let (mut flight, hw) = in_flight();
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    hw.complete();
    assert!(flight.poll_complete(&mut cx).is_ready());
    let wakes_at_recovery = counter.wakes.load(Ordering::SeqCst);

    hw.complete();
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        wakes_at_recovery,
        "late completion wakes nobody"
    );
    assert!(
        flight.poll_complete(&mut cx).is_pending(),
        "spent adapter stays inert"
    );
}

/// Reviewer correction 8: transfer N recovery feeds transfer N+1 — the
/// recovered transport and buffer start a second flight that completes
/// independently, and the spare's identity survives both flights.
#[test]
fn recovered_resources_start_the_next_transfer() {
    let (mut flight, hw) = in_flight();
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    hw.complete();
    let first = match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("first flight must settle"),
    };

    let (second_transfer, second_hw) = start_model_transfer(first.transport, first.buffer);
    let mut second_flight = InFlight::new(second_transfer, first.spare);
    assert!(second_flight.poll_complete(&mut cx).is_pending());
    second_hw.complete();
    match second_flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => {
            assert_eq!(settled.outcome, TransferOutcome::Completed);
            assert_eq!(settled.transport.0, "qspi", "same transport identity");
            assert_eq!(settled.spare.0, "spare-0", "same spare identity");
        }
        Poll::Pending => panic!("second flight must settle"),
    }
}

#[test]
fn spare_is_writable_during_flight_and_drain_flag_clears() {
    let (mut flight, hw) = in_flight();
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert!(flight.spare_mut().is_some(), "spare writable in flight");

    flight.begin_drain();
    assert!(flight.is_draining());
    assert!(flight.poll_complete(&mut cx).is_ready());
    assert!(
        !flight.is_draining(),
        "a settled adapter is no longer draining (reviewer correction 6)"
    );
    let _ = hw;
}

// ---------------------------------------------------------------------------
// Negative control (reviewer-requested): a check-then-register
// implementation LOSES the completion that lands between the check and the
// registration. This test asserts the defect occurs, proving the adversarial
// oracle above is load-bearing and the mandated order is not ceremony.
// ---------------------------------------------------------------------------

struct BrokenTransfer {
    hw: Arc<ModelHw>,
    settled: bool,
}

impl BrokenTransfer {
    fn poll_done_check_then_register(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.settled {
            return Poll::Ready(());
        }
        // BROKEN ORDER: check first...
        if self.hw.done.load(Ordering::SeqCst) {
            self.settled = true;
            return Poll::Ready(());
        }
        // ...completion lands here (injected deterministically)...
        if self.hw.complete_on_register.swap(false, Ordering::SeqCst) {
            self.hw.done.store(true, Ordering::SeqCst);
            // The "interrupt" fires with no waker registered: wakes nobody.
        }
        // ...then register, too late, with no recheck.
        *self.hw.waker.lock().expect("waker lock") = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[test]
fn negative_control_check_then_register_loses_the_wake() {
    let hw = Arc::new(ModelHw::default());
    hw.complete_on_register.store(true, Ordering::SeqCst);
    let mut broken = BrokenTransfer {
        hw: Arc::clone(&hw),
        settled: false,
    };

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    // The racing completion is missed: pending, and nobody will ever wake.
    assert!(broken.poll_done_check_then_register(&mut cx).is_pending());
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        0,
        "the lost-wake race: done is set, waker registered too late, no wake ever fires"
    );
    assert!(
        hw.done.load(Ordering::SeqCst),
        "hardware really did complete — the event exists and was lost"
    );
}
