#![allow(missing_docs)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use kittens::source::{
    self, BacklogSource, ChannelEvent, DrainableSource, MpscReceiver, ReactorSource, TryNext, close,
};

struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct FutureState {
    ready: AtomicBool,
    polls: AtomicUsize,
    drops: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

struct ManualFuture {
    state: Arc<FutureState>,
    output: u8,
}

impl ManualFuture {
    fn new(state: Arc<FutureState>, output: u8) -> Self {
        Self { state, output }
    }
}

impl Future for ManualFuture {
    type Output = u8;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.state.polls.fetch_add(1, Ordering::SeqCst);
        if self.state.ready.load(Ordering::SeqCst) {
            Poll::Ready(self.output)
        } else {
            *self.state.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for ManualFuture {
    fn drop(&mut self) {
        self.state.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn persistent_mpsc_normalizes_receivers_and_enforces_static_close_policies() {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let (bounded_sender, bounded_receiver) = tokio::sync::mpsc::channel::<u8>(2);
    let mut bounded = source::mpsc(bounded_receiver, close::Dormant);
    assert!(!bounded.is_dormant());
    assert!(!bounded.has_backlog());
    assert_eq!(bounded.poll_next(&mut context), Poll::Pending);

    bounded_sender.send(10).await.unwrap();
    assert!(bounded.has_backlog());
    assert_eq!(bounded.poll_next(&mut context), Poll::Ready(10));
    bounded_sender.send(11).await.unwrap();
    assert_eq!(bounded.try_next(), TryNext::Item(11));
    assert_eq!(bounded.try_next(), TryNext::Empty);

    drop(bounded_sender);
    assert!(!bounded.has_backlog());
    assert_eq!(bounded.poll_next(&mut context), Poll::Pending);
    assert!(bounded.is_dormant());
    assert_eq!(bounded.poll_next(&mut context), Poll::Pending);
    assert_eq!(bounded.try_next(), TryNext::Dormant);
    assert!(!bounded.has_backlog());

    let (bounded_sender, bounded_receiver) = tokio::sync::mpsc::channel::<u8>(1);
    drop(bounded_sender);
    let mut bounded_emit = source::mpsc(bounded_receiver, close::Emit);
    assert!(bounded_emit.has_backlog());
    assert_eq!(bounded_emit.try_next(), TryNext::Item(ChannelEvent::Closed));
    assert!(bounded_emit.is_dormant());
    assert_eq!(bounded_emit.try_next(), TryNext::Dormant);

    let (unbounded_sender, unbounded_receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let mut unbounded = source::mpsc(unbounded_receiver, close::Emit);
    assert!(!unbounded.has_backlog());
    assert_eq!(unbounded.poll_next(&mut context), Poll::Pending);
    unbounded_sender.send(20).unwrap();
    unbounded_sender.send(21).unwrap();
    assert!(unbounded.has_backlog());
    assert_eq!(
        unbounded.poll_next(&mut context),
        Poll::Ready(ChannelEvent::Item(20))
    );
    assert_eq!(unbounded.try_next(), TryNext::Item(ChannelEvent::Item(21)));
    assert_eq!(unbounded.try_next(), TryNext::Empty);
    drop(unbounded_sender);
    assert!(unbounded.has_backlog());
    assert_eq!(
        unbounded.poll_next(&mut context),
        Poll::Ready(ChannelEvent::Closed)
    );
    assert_eq!(unbounded.poll_next(&mut context), Poll::Pending);
    assert_eq!(unbounded.try_next(), TryNext::Dormant);

    let (unbounded_sender, unbounded_receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    drop(unbounded_sender);
    let mut unbounded_dormant = source::mpsc(unbounded_receiver, close::Dormant);
    assert_eq!(unbounded_dormant.try_next(), TryNext::Dormant);
    assert!(unbounded_dormant.is_dormant());
}

#[tokio::test]
async fn exhausted_budget_defers_mpsc_drain_without_consuming_the_item() {
    let (sender, receiver) = tokio::sync::mpsc::channel::<u8>(1);
    sender.send(1).await.unwrap();
    let mut source = source::mpsc(receiver, close::Dormant);

    while tokio::task::coop::has_budget_remaining() {
        tokio::task::consume_budget().await;
    }
    assert_eq!(source.try_next(), TryNext::Empty);

    tokio::task::yield_now().await;
    assert_eq!(source.try_next(), TryNext::Item(1));
}

#[test]
fn mpsc_repoll_replaces_the_registered_waker_for_both_receiver_variants() {
    let first_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let second_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let first_registration = Waker::from(Arc::clone(&first_wake_count));
    let second_registration = Waker::from(Arc::clone(&second_wake_count));
    let mut first_context = Context::from_waker(&first_registration);
    let mut second_context = Context::from_waker(&second_registration);

    let (bounded_sender, bounded_receiver) = tokio::sync::mpsc::channel::<u8>(1);
    let mut bounded = source::mpsc(bounded_receiver, close::Dormant);
    assert_eq!(bounded.poll_next(&mut first_context), Poll::Pending);
    assert_eq!(bounded.poll_next(&mut second_context), Poll::Pending);
    bounded_sender.try_send(1).unwrap();
    assert_eq!(first_wake_count.0.load(Ordering::SeqCst), 0);
    assert_eq!(second_wake_count.0.load(Ordering::SeqCst), 1);
    assert_eq!(bounded.poll_next(&mut second_context), Poll::Ready(1));

    let first_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let second_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let first_registration = Waker::from(Arc::clone(&first_wake_count));
    let second_registration = Waker::from(Arc::clone(&second_wake_count));
    let mut first_context = Context::from_waker(&first_registration);
    let mut second_context = Context::from_waker(&second_registration);
    let (unbounded_sender, unbounded_receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let mut unbounded = source::mpsc(unbounded_receiver, close::Dormant);
    assert_eq!(unbounded.poll_next(&mut first_context), Poll::Pending);
    assert_eq!(unbounded.poll_next(&mut second_context), Poll::Pending);
    unbounded_sender.send(2).unwrap();
    assert_eq!(first_wake_count.0.load(Ordering::SeqCst), 0);
    assert_eq!(second_wake_count.0.load(Ordering::SeqCst), 1);
    assert_eq!(unbounded.poll_next(&mut second_context), Poll::Ready(2));
}

#[test]
fn dropping_persistent_mpsc_closes_the_receiver_and_drops_buffered_items() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    sender.try_send(DropProbe(Arc::clone(&drops))).unwrap();
    let source = source::mpsc(receiver, close::Dormant);

    drop(source);

    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(sender.try_send(DropProbe(Arc::clone(&drops))).is_err());
    drop(sender);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn optional_mpsc_returns_rejected_and_replaced_generations_to_the_caller() {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut source = source::OptionalMpsc::new(close::Dormant);

    let (first_sender, first_receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    assert!(source.arm(first_receiver).is_ok());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(!source.has_backlog());

    let (rejected_unbounded_sender, rejected_unbounded_receiver) =
        tokio::sync::mpsc::unbounded_channel::<u8>();
    rejected_unbounded_sender.send(6).unwrap();
    let error = source.arm(rejected_unbounded_receiver).unwrap_err();
    match error.into_inner() {
        MpscReceiver::Unbounded(mut receiver) => assert_eq!(receiver.try_recv(), Ok(6)),
        MpscReceiver::Bounded(_) => panic!("the rejected unbounded generation changed type"),
    }

    let (rejected_sender, rejected_receiver) = tokio::sync::mpsc::channel::<u8>(1);
    rejected_sender.try_send(7).unwrap();
    let error = source.arm(rejected_receiver).unwrap_err();
    match error.into_inner() {
        MpscReceiver::Bounded(mut receiver) => assert_eq!(receiver.try_recv(), Ok(7)),
        MpscReceiver::Unbounded(_) => panic!("the rejected bounded generation changed type"),
    }

    first_sender.send(1).unwrap();
    assert!(source.has_backlog());
    let (replacement_sender, replacement_receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    replacement_sender.send(2).unwrap();
    let previous = source
        .replace(replacement_receiver)
        .expect("the installed generation must be returned");
    match previous {
        MpscReceiver::Unbounded(mut receiver) => assert_eq!(receiver.try_recv(), Ok(1)),
        MpscReceiver::Bounded(_) => panic!("the original unbounded generation changed type"),
    }
    assert_eq!(source.poll_next(&mut context), Poll::Ready(2));

    replacement_sender.send(3).unwrap();
    let returned = source
        .disarm()
        .expect("disarm must return the live replacement");
    assert!(source.is_dormant());
    match returned {
        MpscReceiver::Unbounded(mut receiver) => assert_eq!(receiver.try_recv(), Ok(3)),
        MpscReceiver::Bounded(_) => panic!("the replacement generation changed type"),
    }

    let (last_sender, last_receiver) = tokio::sync::mpsc::channel::<u8>(1);
    last_sender.try_send(4).unwrap();
    assert!(source.replace(last_receiver).is_none());
    assert_eq!(source.poll_next(&mut context), Poll::Ready(4));
    drop(last_sender);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(source.is_dormant());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    let (_sender, receiver) = tokio::sync::mpsc::channel::<u8>(1);
    assert!(source.arm(receiver).is_ok());
    assert!(source.disarm().is_some());
}

#[test]
fn optional_mpsc_backlog_includes_one_emit_close_event() {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let (sender, receiver) = tokio::sync::mpsc::channel::<u8>(1);
    let mut source = source::OptionalMpsc::new(close::Emit);
    assert!(source.arm(receiver).is_ok());
    let (_rejected_sender, rejected_receiver) = tokio::sync::mpsc::channel::<u8>(1);
    assert!(source.arm(rejected_receiver).is_err());

    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    sender.try_send(8).unwrap();
    assert!(source.has_backlog());
    assert_eq!(
        source.poll_next(&mut context),
        Poll::Ready(ChannelEvent::Item(8))
    );
    assert!(!source.has_backlog());
    drop(sender);
    assert!(source.has_backlog());
    assert_eq!(
        source.poll_next(&mut context),
        Poll::Ready(ChannelEvent::Closed)
    );
    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
}

#[test]
fn retained_one_shot_preserves_pending_state_and_replaces_its_registered_waker() {
    let state = Arc::new(FutureState::default());
    let first_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let second_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let first_registration = Waker::from(Arc::clone(&first_wake_count));
    let second_registration = Waker::from(Arc::clone(&second_wake_count));
    let mut first_context = Context::from_waker(&first_registration);
    let mut second_context = Context::from_waker(&second_registration);
    let mut source = source::one_shot(ManualFuture::new(Arc::clone(&state), 9));

    assert_eq!(source.poll_next(&mut first_context), Poll::Pending);
    assert_eq!(source.poll_next(&mut second_context), Poll::Pending);
    assert_eq!(state.polls.load(Ordering::SeqCst), 2);

    state.ready.store(true, Ordering::SeqCst);
    state.waker.lock().unwrap().as_ref().unwrap().wake_by_ref();
    assert_eq!(first_wake_count.0.load(Ordering::SeqCst), 0);
    assert_eq!(second_wake_count.0.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut second_context), Poll::Ready(9));
    assert_eq!(state.drops.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut second_context), Poll::Pending);
}

#[test]
fn dropping_a_pending_one_shot_drops_the_retained_operation_once() {
    let state = Arc::new(FutureState::default());
    let mut source = source::one_shot(ManualFuture::new(Arc::clone(&state), 1));
    let registration = Waker::noop();
    let mut context = Context::from_waker(registration);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    drop(source);

    assert_eq!(state.polls.load(Ordering::SeqCst), 1);
    assert_eq!(state.drops.load(Ordering::SeqCst), 1);
}

#[test]
fn optional_one_shot_makes_arming_replacement_and_cancellation_explicit() {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut source = source::OptionalOneShot::<ManualFuture>::default();
    assert!(source.is_dormant());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    let first = Arc::new(FutureState::default());
    assert!(source.arm(ManualFuture::new(Arc::clone(&first), 1)).is_ok());
    let rejected = Arc::new(FutureState::default());
    let rejected_future = source
        .arm(ManualFuture::new(Arc::clone(&rejected), 2))
        .unwrap_err()
        .into_inner();
    assert!(Arc::ptr_eq(&rejected_future.state, &rejected));
    drop(rejected_future);
    assert_eq!(rejected.drops.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    let replacement = Arc::new(FutureState::default());
    replacement.ready.store(true, Ordering::SeqCst);
    assert!(source.cancel_and_replace(ManualFuture::new(Arc::clone(&replacement), 3)));
    assert_eq!(first.drops.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut context), Poll::Ready(3));
    assert!(source.is_dormant());
    assert!(!source.cancel_and_disarm());

    let canceled = Arc::new(FutureState::default());
    assert!(
        source
            .arm(ManualFuture::new(Arc::clone(&canceled), 4))
            .is_ok()
    );
    assert!(source.cancel_and_disarm());
    assert_eq!(canceled.drops.load(Ordering::SeqCst), 1);
    assert!(source.is_dormant());

    let installed = Arc::new(FutureState::default());
    assert!(!source.cancel_and_replace(ManualFuture::new(Arc::clone(&installed), 5)));
    assert!(!source.is_dormant());
}

#[test]
fn optional_one_shot_from_future_disarms_before_returning_ready() {
    let state = Arc::new(FutureState::default());
    state.ready.store(true, Ordering::SeqCst);
    let mut source = source::OptionalOneShot::from_future(ManualFuture::new(Arc::clone(&state), 6));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(source.poll_next(&mut context), Poll::Ready(6));
    assert!(source.is_dormant());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
}

#[test]
fn cancellation_replaces_registration_and_becomes_dormant_after_delivery() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut source = source::cancellation(token.clone());
    let first_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let second_wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let first_registration = Waker::from(Arc::clone(&first_wake_count));
    let second_registration = Waker::from(Arc::clone(&second_wake_count));
    let mut first_context = Context::from_waker(&first_registration);
    let mut second_context = Context::from_waker(&second_registration);

    assert_eq!(source.poll_next(&mut first_context), Poll::Pending);
    assert_eq!(source.poll_next(&mut second_context), Poll::Pending);
    token.cancel();
    assert_eq!(first_wake_count.0.load(Ordering::SeqCst), 0);
    assert_eq!(second_wake_count.0.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut second_context), Poll::Ready(()));
    assert_eq!(source.poll_next(&mut second_context), Poll::Pending);
}

#[test]
fn dropping_a_cancellation_waiter_does_not_cancel_other_token_owners() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut source = source::cancellation(token.clone());
    let wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let registration = Waker::from(Arc::clone(&wake_count));
    let mut context = Context::from_waker(&registration);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    drop(source);
    assert!(!token.is_cancelled());
    token.cancel();

    assert!(token.is_cancelled());
    assert_eq!(wake_count.0.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn optional_deadline_resets_disarms_and_reports_its_absolute_instant() {
    let mut source = source::OptionalDeadline::default();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let now = tokio::time::Instant::now();
    let original = now + std::time::Duration::from_secs(10);
    let reset = now + std::time::Duration::from_secs(3);

    assert!(source.is_dormant());
    assert_eq!(source.deadline(), None);
    source.set_at(original);
    assert_eq!(source.deadline(), Some(original));
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    source.set_at(reset);
    assert_eq!(source.deadline(), Some(reset));
    tokio::time::advance(std::time::Duration::from_secs(3)).await;
    assert_eq!(source.poll_next(&mut context), Poll::Ready(reset));
    assert!(source.is_dormant());
    assert_eq!(source.deadline(), None);

    source.set_at(now + std::time::Duration::from_secs(20));
    source.set(None);
    assert!(source.is_dormant());
    assert_eq!(source.deadline(), None);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    source.set_at(now + std::time::Duration::from_secs(30));
    source.disarm();
    assert!(source.is_dormant());
    assert_eq!(source.deadline(), None);
}

#[tokio::test(start_paused = true)]
async fn dropping_an_armed_deadline_removes_its_timer_registration() {
    let wake_count = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let registration = Waker::from(Arc::clone(&wake_count));
    let mut context = Context::from_waker(&registration);
    let mut source = source::OptionalDeadline::new();
    source.set_at(tokio::time::Instant::now() + std::time::Duration::from_secs(1));
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    drop(source);
    tokio::time::advance(std::time::Duration::from_secs(1)).await;
    tokio::task::yield_now().await;

    assert_eq!(wake_count.0.load(Ordering::SeqCst), 0);
}
