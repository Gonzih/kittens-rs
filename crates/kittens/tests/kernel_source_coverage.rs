#![allow(missing_docs)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use kittens::reactor::Control;
use kittens::source::{
    BacklogSource, ChannelClosePolicy, ChannelEvent, DrainableSource, FixedQueue, Latched,
    ReactorSource, TryNext, close, readiness,
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

    kittens::__private::assert_SRC001_reactor_source_is_admitted__repair_use_retained_or_channel(
        &latch,
    );
    kittens::__private::assert_KTR006_declared_readiness_matches::<readiness::Quiescent, _>(&latch);
    kittens::__private::assert_KTR009_source_is_drainable(&queue);
    kittens::__private::assert_KTR010_yield_target_has_backlog_probe(&latch);

    assert!(latch.has_backlog());
    assert_eq!(queue.len(), 1);
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
