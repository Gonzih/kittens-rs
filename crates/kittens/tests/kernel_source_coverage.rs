#![allow(missing_docs)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use kittens::reactor::Control;
use kittens::source::{
    BacklogSource, ChannelClosePolicy, ChannelEvent, DrainableSource, FixedQueue, Latched,
    OptionalInlineOneShot, ReactorSource, TryNext, close, readiness,
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

fn counting_context() -> (Arc<WakeCounter>, Waker) {
    let counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&counter));
    (counter, waker)
}

#[derive(Default)]
struct FutureState {
    ready: AtomicBool,
    polls: AtomicUsize,
    drops: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

impl FutureState {
    fn complete(&self) {
        self.ready.store(true, Ordering::SeqCst);
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }
}

struct TrackedFuture {
    output: u8,
    state: Arc<FutureState>,
}

impl TrackedFuture {
    fn new(output: u8) -> (Self, Arc<FutureState>) {
        let state = Arc::new(FutureState::default());
        (
            Self {
                output,
                state: Arc::clone(&state),
            },
            state,
        )
    }
}

impl Future for TrackedFuture {
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

impl Drop for TrackedFuture {
    fn drop(&mut self) {
        self.state.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn close_policies_map_items_and_expose_only_the_configured_close_event() {
    const {
        assert!(!<close::Dormant as ChannelClosePolicy<u8>>::EMITS_CLOSE);
    }
    assert_eq!(<close::Dormant as ChannelClosePolicy<u8>>::map_item(7), 7);
    assert_eq!(
        <close::Dormant as ChannelClosePolicy<u8>>::close_event(),
        None
    );

    const {
        assert!(<close::Emit as ChannelClosePolicy<u8>>::EMITS_CLOSE);
    }
    assert_eq!(
        <close::Emit as ChannelClosePolicy<u8>>::map_item(8),
        ChannelEvent::Item(8)
    );
    assert_eq!(
        <close::Emit as ChannelClosePolicy<u8>>::close_event(),
        Some(ChannelEvent::Closed)
    );
}

#[test]
fn latched_preserves_the_first_event_and_dormant_polls_do_not_self_wake() {
    let mut source = Latched::new();
    let (wake_counter, waker) = counting_context();
    let mut context = Context::from_waker(&waker);

    assert!(source.is_dormant());
    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    source.arm(11).expect("a dormant latch accepts an event");
    assert!(!source.is_dormant());
    assert!(source.has_backlog());
    let rejected = source
        .arm(12)
        .expect_err("an armed latch rejects replacement");
    assert_eq!(rejected.into_inner(), 12);
    assert_eq!(source.poll_next(&mut context), Poll::Ready(11));
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(source.is_dormant());
    assert!(!source.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    source.arm(13).expect("a delivered latch can be rearmed");
    assert_eq!(source.disarm(), Some(13));
    assert!(source.is_dormant());
}

#[test]
fn inline_one_shot_dormancy_does_not_self_wake() {
    let mut source = OptionalInlineOneShot::<TrackedFuture>::new();
    let (wake_counter, waker) = counting_context();
    let mut context = Context::from_waker(&waker);

    assert!(source.is_dormant());
    assert!(source.future_mut().is_none());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);
}

#[test]
fn inline_one_shot_rejects_replacement_without_dropping_either_future() {
    let (installed, installed_state) = TrackedFuture::new(21);
    let (replacement, replacement_state) = TrackedFuture::new(22);
    let mut source = OptionalInlineOneShot::from_future(installed);

    let rejected = source
        .arm(replacement)
        .expect_err("an armed carrier rejects replacement")
        .into_inner();
    assert_eq!(rejected.output, 22);
    assert_eq!(source.future_mut().unwrap().output, 21);
    assert_eq!(installed_state.drops.load(Ordering::SeqCst), 0);
    assert_eq!(replacement_state.drops.load(Ordering::SeqCst), 0);

    drop(source);
    assert_eq!(installed_state.drops.load(Ordering::SeqCst), 1);
    assert_eq!(replacement_state.drops.load(Ordering::SeqCst), 0);
    drop(rejected);
    assert_eq!(replacement_state.drops.load(Ordering::SeqCst), 1);
}

#[test]
fn inline_one_shot_retains_pending_work_replaces_its_waker_and_rearms() {
    let (first, first_state) = TrackedFuture::new(31);
    let mut source = OptionalInlineOneShot::from_future(first);
    let (abandoned, old_waker) = counting_context();
    let (notified, current_waker) = counting_context();
    let mut old_poll_context = Context::from_waker(&old_waker);
    let mut new_poll_context = Context::from_waker(&current_waker);

    assert_eq!(source.poll_next(&mut old_poll_context), Poll::Pending);
    assert_eq!(source.poll_next(&mut new_poll_context), Poll::Pending);
    assert_eq!(first_state.polls.load(Ordering::SeqCst), 2);

    first_state.complete();
    assert_eq!(abandoned.0.load(Ordering::SeqCst), 0);
    assert_eq!(notified.0.load(Ordering::SeqCst), 1);
    assert_eq!(source.poll_next(&mut new_poll_context), Poll::Ready(31));
    assert!(source.is_dormant());
    assert!(source.future_mut().is_none());
    assert_eq!(first_state.drops.load(Ordering::SeqCst), 1);

    let (second, second_state) = TrackedFuture::new(32);
    second_state.complete();
    assert!(source.arm(second).is_ok());
    assert_eq!(source.poll_next(&mut new_poll_context), Poll::Ready(32));
    assert!(source.is_dormant());
    assert_eq!(second_state.drops.load(Ordering::SeqCst), 1);

    drop(source);
    assert_eq!(first_state.drops.load(Ordering::SeqCst), 1);
    assert_eq!(second_state.drops.load(Ordering::SeqCst), 1);
}

#[test]
fn fixed_queue_preserves_fifo_order_across_wrap_full_and_empty_transitions() {
    let mut source = FixedQueue::<u8, 3>::default();
    let (wake_counter, waker) = counting_context();
    let mut context = Context::from_waker(&waker);

    assert_eq!(source.len(), 0);
    assert!(source.is_empty());
    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    source.push(10).unwrap();
    source.push(20).unwrap();
    source.push(30).unwrap();
    assert_eq!(source.len(), 3);
    assert!(source.has_backlog());
    assert_eq!(source.push(99).unwrap_err().into_inner(), 99);

    assert_eq!(source.poll_next(&mut context), Poll::Ready(10));
    source
        .push(40)
        .expect("the freed ring slot accepts an item");
    assert_eq!(source.try_next(), TryNext::Item(20));
    assert_eq!(source.try_next(), TryNext::Item(30));
    assert_eq!(source.try_next(), TryNext::Item(40));
    assert_eq!(source.try_next(), TryNext::Empty);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(source.len(), 0);
    assert!(source.is_empty());
    assert!(!source.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    let mut zero = FixedQueue::<u8, 0>::new();
    assert_eq!(zero.push(77).unwrap_err().into_inner(), 77);
    assert_eq!(zero.try_next(), TryNext::Empty);
    assert_eq!(zero.poll_next(&mut context), Poll::Pending);
    assert_eq!(zero.len(), 0);
    assert!(zero.is_empty());
    assert!(!zero.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);
}

#[test]
fn generated_type_assertions_admit_capabilities_without_mutating_sources() {
    let mut latch = Latched::new();
    latch.arm(1).unwrap();
    let mut queue = FixedQueue::<u8, 2>::new();
    queue.push(2).unwrap();
    let inline = OptionalInlineOneShot::from_future(std::future::ready(3_u8));

    kittens::__private::assert_SRC001_reactor_source_is_admitted__repair_use_retained_or_channel(
        &latch,
    );
    kittens::__private::assert_KTR006_declared_readiness_matches::<readiness::Quiescent, _>(&latch);
    kittens::__private::assert_KTR009_source_is_drainable(&queue);
    kittens::__private::assert_KTR010_yield_target_has_backlog_probe(&latch);
    kittens::__private::assert_SRC001_reactor_source_is_admitted__repair_use_retained_or_channel(
        &inline,
    );
    kittens::__private::assert_KTR006_declared_readiness_matches::<readiness::Quiescent, _>(
        &inline,
    );

    assert!(latch.has_backlog());
    assert_eq!(queue.len(), 1);
    assert!(!inline.is_dormant());
}

#[test]
fn generated_value_assertions_preserve_guard_handler_and_phase_results() {
    assert!(kittens::__private::assert_KTR019_guard_result_is_bool(true));
    assert!(!kittens::__private::assert_KTR019_guard_result_is_bool(
        false
    ));

    let continuing: Result<Control<u8>, &str> = Ok(Control::Continue);
    let stopping: Result<Control<u8>, &str> = Ok(Control::Stop(9));
    let handler_error: Result<Control<u8>, &str> = Err("handler");
    assert_eq!(
        kittens::__private::assert_KTR013_continuing_handler_result(continuing),
        Ok(Control::Continue)
    );
    assert_eq!(
        kittens::__private::assert_KTR013_continuing_handler_result(stopping),
        Ok(Control::Stop(9))
    );
    assert_eq!(
        kittens::__private::assert_KTR013_continuing_handler_result(handler_error),
        Err("handler")
    );

    assert_eq!(
        kittens::__private::assert_KTR013_terminal_handler_result::<u8, &str>(Ok(4)),
        Ok(4)
    );
    assert_eq!(
        kittens::__private::assert_KTR013_terminal_handler_result::<u8, &str>(Err("terminal")),
        Err("terminal")
    );
    assert_eq!(
        kittens::__private::assert_KTR013_phase_result::<&str>(Ok(())),
        Ok(())
    );
    assert_eq!(
        kittens::__private::assert_KTR013_phase_result::<&str>(Err("phase")),
        Err("phase")
    );
}
