//! K2R-0A candidate A′ trace oracles (SPEC section 7 pass criteria, host
//! model). The model mirrors the esp-hal owning-transfer shape: start
//! consumes transport + buffer; completion is externally driven (the model's
//! "interrupt"); cancellation settles the transfer; recovery returns
//! everything. Wake counts are the busy-poll oracle.

#![allow(missing_docs)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

/// Counts wakes so oracles can assert exactly when and how often the
/// adapter arranged progress.
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

/// The model "hardware": externally completable, like a transfer-done
/// interrupt. Registration is level-correct: completing after a poll wakes
/// the registered waker exactly once; completing before any poll simply
/// leaves the state ready.
#[derive(Default)]
struct ModelHw {
    done: AtomicBool,
    fail: AtomicBool,
    cancelled: AtomicBool,
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

impl OwnedTransfer for ModelTransfer {
    type Transport = ModelTransport;
    type Buffer = ModelBuffer;

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<TransferOutcome> {
        if let Some(outcome) = self.settled {
            return Poll::Ready(outcome);
        }
        // Cancellation settles the model immediately (real hardware may take
        // a bounded time; the adapter contract only requires settlement
        // through polling).
        if self.hw.cancelled.load(Ordering::SeqCst) && !self.hw.done.load(Ordering::SeqCst) {
            self.settled = Some(TransferOutcome::Cancelled);
            return Poll::Ready(TransferOutcome::Cancelled);
        }
        if self.hw.done.load(Ordering::SeqCst) {
            let outcome = if self.hw.fail.load(Ordering::SeqCst) {
                TransferOutcome::Failed
            } else {
                TransferOutcome::Completed
            };
            self.settled = Some(outcome);
            return Poll::Ready(outcome);
        }
        *self.hw.waker.lock().expect("waker lock") = Some(cx.waker().clone());
        Poll::Pending
    }

    fn cancel(&mut self) {
        self.hw.cancelled.store(true, Ordering::SeqCst);
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

fn in_flight() -> (InFlight<ModelTransfer>, Arc<ModelHw>) {
    let (transfer, hw) = start_model_transfer(ModelTransport("qspi"), ModelBuffer(vec![0xAB; 16]));
    (InFlight::new(transfer), hw)
}

/// Selection-loss position 1: the completion is polled (registers a waker),
/// another source wins, hardware completes — exactly one wake reaches the
/// registered waker and the next poll recovers everything.
#[test]
fn polled_then_lost_arbitration_gets_exactly_one_wake() {
    let (mut flight, hw) = in_flight();
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        0,
        "no self-wake while pending"
    );

    hw.complete();
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        1,
        "completion wakes exactly once"
    );

    match flight.poll_complete(&mut cx) {
        Poll::Ready(recovered) => {
            assert_eq!(recovered.outcome, TransferOutcome::Completed);
            assert_eq!(recovered.transport.0, "qspi");
            assert_eq!(recovered.buffer.0.len(), 16);
        }
        Poll::Pending => panic!("completed transfer must recover"),
    }
    assert!(flight.is_spent());
}

/// Selection-loss position 2: hardware completes before the adapter is ever
/// polled (an earlier source won first). The first poll is immediately
/// ready; nothing was lost by not being polled.
#[test]
fn unpolled_below_winner_recovers_on_first_poll() {
    let (mut flight, hw) = in_flight();
    hw.complete();

    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_ready());
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        0,
        "a ready-before-poll transfer never needed the waker"
    );
}

/// Cancel-and-drain: the drain is requested, the transfer settles through
/// polling, and every resource comes back with the Cancelled outcome.
#[test]
fn cancel_and_drain_returns_all_resources() {
    let (mut flight, _hw) = in_flight();
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(flight.poll_complete(&mut cx).is_pending());
    flight.begin_drain();
    assert!(flight.is_draining());

    match flight.poll_complete(&mut cx) {
        Poll::Ready(recovered) => {
            assert_eq!(recovered.outcome, TransferOutcome::Cancelled);
            assert_eq!(recovered.transport.0, "qspi", "transport recovered");
            assert_eq!(recovered.buffer.0.len(), 16, "buffer recovered");
        }
        Poll::Pending => panic!("a drained transfer must settle"),
    }
}

/// A cancellation racing an already-completed transfer resolves as
/// Completed: the work was done; drain does not rewrite history.
#[test]
fn drain_racing_completion_reports_completed() {
    let (mut flight, hw) = in_flight();
    hw.complete();
    flight.begin_drain();

    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    match flight.poll_complete(&mut cx) {
        Poll::Ready(recovered) => assert_eq!(recovered.outcome, TransferOutcome::Completed),
        Poll::Pending => panic!("settled transfer must recover"),
    }
}

/// Transport failure settles the transfer, returns the resources, and
/// reports Failed — the SPEC 5.3 full-repaint obligation is the caller's
/// next move, not a lost buffer.
#[test]
fn failure_settles_and_returns_resources() {
    let (mut flight, hw) = in_flight();
    let (_counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_pending());

    hw.fail();
    match flight.poll_complete(&mut cx) {
        Poll::Ready(recovered) => {
            assert_eq!(recovered.outcome, TransferOutcome::Failed);
            assert_eq!(recovered.buffer.0.len(), 16);
        }
        Poll::Pending => panic!("failed transfer must settle"),
    }
}

/// A spent adapter stays inert: further polls are Pending and register no
/// wake — the reactor moving on is the caller's job, and a spent slot must
/// not hot-loop.
#[test]
fn spent_adapter_is_inert() {
    let (mut flight, hw) = in_flight();
    hw.complete();
    let (counter, waker) = counting_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(flight.poll_complete(&mut cx).is_ready());

    assert!(flight.poll_complete(&mut cx).is_pending());
    assert!(flight.poll_complete(&mut cx).is_pending());
    assert_eq!(
        counter.wakes.load(Ordering::SeqCst),
        0,
        "a spent adapter never wakes"
    );
}
