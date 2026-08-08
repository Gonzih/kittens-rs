#![allow(clippy::ignored_unit_patterns, missing_docs)]

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use kittens::reactor::Control;
use kittens::source::{
    self, BacklogSource, ChannelEvent, FixedQueue, Latched, ReactorSource, close,
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
struct WakeOnlyState {
    ready: AtomicBool,
    polls: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

struct WakeOnlyFuture(Arc<WakeOnlyState>);

impl Future for WakeOnlyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.polls.fetch_add(1, Ordering::SeqCst);
        if self.0.ready.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            *self.0.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TracePoll {
    Pending(u8),
    Ready(u8),
}

struct TraceState {
    id: u8,
    value: u8,
    ready: AtomicBool,
    registrations: AtomicUsize,
    trace: Arc<Mutex<Vec<TracePoll>>>,
    waker: Mutex<Option<Waker>>,
}

struct TraceFuture(Arc<TraceState>);

impl Future for TraceFuture {
    type Output = u8;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.ready.load(Ordering::SeqCst) {
            self.0
                .trace
                .lock()
                .unwrap()
                .push(TracePoll::Ready(self.0.id));
            Poll::Ready(self.0.value)
        } else {
            self.0
                .trace
                .lock()
                .unwrap()
                .push(TracePoll::Pending(self.0.id));
            self.0.registrations.fetch_add(1, Ordering::SeqCst);
            *self.0.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[test]
fn closed_optional_mpsc_becomes_dormant_without_self_waking() {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let mut source = source::OptionalMpsc::new(close::Dormant);
    source.arm(receiver).expect("initial arm");
    drop(sender);

    let wake_counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);

    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(source.is_dormant());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);
}

#[test]
fn emit_close_policy_yields_closed_exactly_once() {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<u8>();
    let mut source = source::OptionalMpsc::new(close::Emit);
    source.arm(receiver).expect("initial arm");
    drop(sender);

    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(
        source.poll_next(&mut context),
        Poll::Ready(ChannelEvent::Closed)
    );
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(source.is_dormant());
    assert!(!source.has_backlog());
}

#[tokio::test(start_paused = true)]
async fn optional_deadline_disarms_before_delivering() {
    let mut source = source::OptionalDeadline::new();
    let at = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    source.set_at(at);

    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert_eq!(source.poll_next(&mut context), Poll::Ready(at));
    assert!(source.is_dormant());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
}

#[tokio::test(start_paused = true)]
async fn absolute_deadline_survives_a_lost_race_without_relative_reconstruction() {
    struct Sources {
        deadline: source::OptionalDeadline,
        earlier_winner: Latched<()>,
    }

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut deadline_source = source::OptionalDeadline::new();
    deadline_source.set_at(deadline);
    let mut sources = Sources {
        deadline: deadline_source,
        earlier_winner: Latched::new(),
    };
    sources.earlier_winner.arm(()).unwrap();
    let mut order = Vec::new();

    let result: Result<tokio::time::Instant, Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(deadline)]
        #[readiness(quiescent)]
        #[terminal]
        fired_at = sources.deadline => {
            order.push(2);
            Ok(fired_at)
        }

        #[source(earlier_winner)]
        #[readiness(quiescent)]
        _ = sources.earlier_winner => {
            order.push(1);
            tokio::time::advance(std::time::Duration::from_secs(5)).await;
            Ok(Control::Continue)
        }
    };

    assert_eq!(result, Ok(deadline));
    assert_eq!(order, [1, 2]);
}

#[derive(Debug, Eq, PartialEq)]
enum Exit {
    Done,
}

struct LostRaceSources<F: Future<Output = Result<u8, tokio::sync::oneshot::error::RecvError>>> {
    retained: source::OneShot<F>,
    earlier_winner: Latched<()>,
}

#[tokio::test]
async fn retained_one_shot_survives_being_polled_before_another_source_wins() {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let mut sources = LostRaceSources {
        retained: source::one_shot(receiver),
        earlier_winner: Latched::new(),
    };
    sources.earlier_winner.arm(()).unwrap();
    let mut sender = Some(sender);
    let mut selections = Vec::new();

    let exit: Result<Exit, tokio::sync::oneshot::error::RecvError> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(retained)]
        #[readiness(quiescent)]
        #[terminal]
        value = sources.retained => {
            selections.push(value?);
            Ok(Exit::Done)
        }

        #[source(earlier_winner)]
        #[readiness(quiescent)]
        _ = sources.earlier_winner => {
            selections.push(1);
            sender.take().unwrap().send(2).unwrap();
            Ok(Control::Continue)
        }
    };

    assert_eq!(exit, Ok(Exit::Done));
    assert_eq!(selections, [1, 2]);
}

#[tokio::test]
async fn latched_event_survives_when_an_earlier_source_wins_before_it_is_polled() {
    struct Sources {
        first: Latched<u8>,
        second: Latched<u8>,
    }
    let mut sources = Sources {
        first: Latched::new(),
        second: Latched::new(),
    };
    sources.first.arm(1).unwrap();
    sources.second.arm(2).unwrap();
    let mut selections = Vec::new();

    let result: Result<(), Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(first)]
        #[readiness(quiescent)]
        value = sources.first => {
            selections.push(value);
            Ok(Control::Continue)
        }

        #[source(second)]
        #[readiness(quiescent)]
        value = sources.second => {
            selections.push(value);
            Ok(Control::Stop(()))
        }
    };

    assert_eq!(result, Ok(()));
    assert_eq!(selections, [1, 2]);
}

#[tokio::test]
async fn before_poll_runs_once_across_pending_executor_repolls() {
    struct Sources<F: Future<Output = Result<u8, tokio::sync::oneshot::error::RecvError>>> {
        completion: source::OneShot<F>,
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let mut sources = Sources {
        completion: source::one_shot(receiver),
    };
    let mut before_count = 0;
    let mut guard_count = 0;
    let sender_task = tokio::spawn(async move {
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        sender.send(7).unwrap();
    });

    let result: Result<u8, tokio::sync::oneshot::error::RecvError> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [before_poll];
        }

        before_poll {
            before_count += 1;
            Ok(())
        }

        #[source(completion)]
        #[readiness(quiescent)]
        #[when({ guard_count += 1; true })]
        #[terminal]
        value = sources.completion => {
            value
        }
    };

    sender_task.await.unwrap();
    assert_eq!(result, Ok(7));
    assert_eq!(before_count, 1);
    assert_eq!(guard_count, 1);
}

#[tokio::test]
async fn wake_only_guard_change_waits_for_the_next_arbitration() {
    struct Sources {
        guarded: Latched<u8>,
        control: source::OneShot<WakeOnlyFuture>,
    }

    let enabled = Arc::new(AtomicBool::new(false));
    let wake_state = Arc::new(WakeOnlyState::default());
    let mut sources = Sources {
        guarded: Latched::new(),
        control: source::one_shot(WakeOnlyFuture(Arc::clone(&wake_state))),
    };
    sources.guarded.arm(7).unwrap();
    let enabled_in_task = Arc::clone(&enabled);
    let wake_state_in_task = Arc::clone(&wake_state);
    let driver = tokio::spawn(async move {
        while wake_state_in_task.polls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }

        enabled_in_task.store(true, Ordering::SeqCst);
        wake_state_in_task
            .waker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .wake_by_ref();

        while wake_state_in_task.polls.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
        wake_state_in_task.ready.store(true, Ordering::SeqCst);
        wake_state_in_task
            .waker
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .wake_by_ref();
    });

    let mut guard_snapshots = 0;
    let mut selections = Vec::new();
    let result: Result<(), Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(guarded)]
        #[readiness(quiescent)]
        #[when({
            guard_snapshots += 1;
            enabled.load(Ordering::SeqCst)
        })]
        value = sources.guarded => {
            selections.push(value);
            Ok(Control::Stop(()))
        }

        #[source(control)]
        #[readiness(quiescent)]
        _ = sources.control => {
            selections.push(0);
            Ok(Control::Continue)
        }
    };

    driver.await.unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(selections, [0, 7]);
    assert_eq!(guard_snapshots, 2);
    assert!(wake_state.polls.load(Ordering::SeqCst) >= 3);
}

#[tokio::test]
async fn buffered_yield_selects_input_before_a_ready_firehose() {
    struct Sources {
        model: FixedQueue<u8, 8>,
        input: FixedQueue<u8, 2>,
    }
    let mut sources = Sources {
        model: FixedQueue::new(),
        input: FixedQueue::new(),
    };
    sources.model.push(1).unwrap();
    sources.model.push(2).unwrap();
    sources.input.push(9).unwrap();
    let mut order = Vec::new();

    let result: Result<(), Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(model)]
        #[readiness(may_remain_ready)]
        #[yields_to(input, when = buffered)]
        token = sources.model => {
            order.push(token);
            Ok(Control::Stop(()))
        }

        #[source(input)]
        #[readiness(may_remain_ready)]
        input = sources.input => {
            order.push(input);
            Ok(Control::Continue)
        }
    };

    assert_eq!(result, Ok(()));
    assert_eq!(order, [9, 1]);
}

#[tokio::test]
async fn drain_bound_and_after_event_are_one_service_window() {
    struct Sources {
        stream: FixedQueue<u8, 8>,
    }
    let mut sources = Sources {
        stream: FixedQueue::new(),
    };
    for value in 1..=5 {
        sources.stream.push(value).unwrap();
    }
    let mut handled = Vec::new();
    let mut after_windows = Vec::new();

    let result: Result<(), Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [after_event];
        }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "the only stream owns this fixture")]
        #[drain(max = 3)]
        item = sources.stream => {
            handled.push(item);
            if item == 5 {
                Ok(Control::Stop(()))
            } else {
                Ok(Control::Continue)
            }
        }

        after_event {
            after_windows.push(handled.len());
            Ok(())
        }
    };

    assert_eq!(result, Ok(()));
    assert_eq!(handled, [1, 2, 3, 4, 5]);
    assert_eq!(after_windows, [3]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HandlerError;

#[tokio::test]
async fn handler_error_mid_drain_skips_after_event_after_prior_mutation() {
    struct Sources {
        stream: FixedQueue<u8, 4>,
    }
    let mut sources = Sources {
        stream: FixedQueue::new(),
    };
    sources.stream.push(1).unwrap();
    sources.stream.push(2).unwrap();
    let mut handled = Vec::new();
    let mut after = 0;

    let result: Result<(), HandlerError> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [after_event];
        }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "the only stream owns this fixture")]
        #[drain(max = 4)]
        item = sources.stream => {
            handled.push(item);
            if item == 2 {
                Err(HandlerError)
            } else {
                Ok(Control::Continue)
            }
        }

        after_event {
            after += 1;
            Ok(())
        }
    };

    assert_eq!(result, Err(HandlerError));
    assert_eq!(handled, [1, 2]);
    assert_eq!(after, 0);
}

#[tokio::test]
async fn handler_panic_mid_drain_unwinds_without_compensation_or_after_event() {
    let handled = Arc::new(AtomicUsize::new(0));
    let after = Arc::new(AtomicUsize::new(0));
    let handled_in_task = Arc::clone(&handled);
    let after_in_task = Arc::clone(&after);

    let task = tokio::spawn(async move {
        struct Sources {
            stream: FixedQueue<u8, 4>,
        }
        let mut sources = Sources {
            stream: FixedQueue::new(),
        };
        sources.stream.push(1).unwrap();
        sources.stream.push(2).unwrap();

        let _result: Result<(), Infallible> = kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [after_event];
            }

            #[source(stream)]
            #[readiness(may_remain_ready)]
            #[starvation(allowed, reason = "the only stream owns this fixture")]
            #[drain(max = 4)]
            item = sources.stream => {
                handled_in_task.fetch_add(1, Ordering::SeqCst);
                assert_ne!(item, 2, "fixture panic during drain");
                Ok(Control::Continue)
            }

            after_event {
                after_in_task.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        };
    });

    let error = task.await.expect_err("handler panic must unwind the task");
    assert!(error.is_panic());
    assert_eq!(handled.load(Ordering::SeqCst), 2);
    assert_eq!(after.load(Ordering::SeqCst), 0);
}

macro_rules! define_budget_runner {
    ($name:ident, $reactor:ident) => {
        async fn $name() -> (Vec<u8>, Vec<usize>, Vec<(u8, bool)>) {
            struct Sources {
                stream: source::Mpsc<u8, close::Dormant>,
            }
            let (sender, receiver) = tokio::sync::mpsc::channel(64);
            for value in 1..=4 {
                sender.send(value).await.unwrap();
            }
            let mut sources = Sources {
                stream: source::mpsc(receiver, close::Dormant),
            };
            let mut handled = Vec::new();
            let mut windows = Vec::new();
            let mut budget_trace = Vec::new();

            let result: Result<(), Infallible> = kittens::__private::$reactor! {
                policy {
                    selection: biased;
                    required_phases: [after_event];
                }

                #[source(stream)]
                #[readiness(may_remain_ready)]
                #[starvation(allowed, reason = "the only stream owns this fixture")]
                #[drain(max = 32)]
                item = sources.stream => {
                    handled.push(item);
                    budget_trace.push((item, tokio::task::coop::has_budget_remaining()));
                    if item == 1 {
                        while tokio::task::coop::has_budget_remaining() {
                            tokio::task::consume_budget().await;
                        }
                        budget_trace.push((0, tokio::task::coop::has_budget_remaining()));
                    }
                    if item == 4 {
                        Ok(Control::Stop(()))
                    } else {
                        Ok(Control::Continue)
                    }
                }

                after_event {
                    windows.push(handled.len());
                    Ok(())
                }
            };

            drop(sender);
            result.unwrap();
            (handled, windows, budget_trace)
        }
    };
}

define_budget_runner!(run_core_budget, reactor_event);
define_budget_runner!(run_tokio_budget, reactor_tokio_event);

#[tokio::test]
async fn exhausted_tokio_budget_ends_both_mpsc_drain_windows_early() {
    let core = tokio::spawn(run_core_budget()).await.unwrap();
    let control = tokio::spawn(run_tokio_budget()).await.unwrap();
    eprintln!(
        "cooperative-budget trace (item, has_budget): {:?}; windows={:?}",
        core.2, core.1
    );
    assert_eq!(core, control);
    assert_eq!(core.0, [1, 2, 3, 4]);
    assert_eq!(core.1.first(), Some(&1));
    assert_eq!(core.2.first(), Some(&(1, true)));
    assert_eq!(core.2.get(1), Some(&(0, false)));
}

macro_rules! define_trace_runner {
    ($name:ident, $reactor:ident) => {
        async fn $name() -> (Vec<u8>, Vec<TracePoll>, usize) {
            struct Sources {
                first: source::OneShot<TraceFuture>,
                second: source::OneShot<TraceFuture>,
            }

            let trace = Arc::new(Mutex::new(Vec::new()));
            let first_state = Arc::new(TraceState {
                id: 1,
                value: 1,
                ready: AtomicBool::new(false),
                registrations: AtomicUsize::new(0),
                trace: Arc::clone(&trace),
                waker: Mutex::new(None),
            });
            let second_state = Arc::new(TraceState {
                id: 2,
                value: 2,
                ready: AtomicBool::new(true),
                registrations: AtomicUsize::new(0),
                trace: Arc::clone(&trace),
                waker: Mutex::new(None),
            });
            let mut sources = Sources {
                first: source::one_shot(TraceFuture(Arc::clone(&first_state))),
                second: source::one_shot(TraceFuture(second_state)),
            };
            let mut selections = Vec::new();

            let result: Result<(), Infallible> = kittens::__private::$reactor! {
                policy {
                    selection: biased;
                    required_phases: [];
                }

                #[source(first)]
                #[readiness(quiescent)]
                value = sources.first => {
                    selections.push(value);
                    Ok(Control::Stop(()))
                }

                #[source(second)]
                #[readiness(quiescent)]
                value = sources.second => {
                    selections.push(value);
                    first_state.ready.store(true, Ordering::SeqCst);
                    first_state
                        .waker
                        .lock()
                        .unwrap()
                        .as_ref()
                        .expect("first source registered the arbitration waker")
                        .wake_by_ref();
                    Ok(Control::Continue)
                }
            };
            result.unwrap();
            let recorded = trace.lock().unwrap().clone();
            (
                selections,
                recorded,
                first_state.registrations.load(Ordering::SeqCst),
            )
        }
    };
}

define_trace_runner!(run_core_trace, reactor_event);
define_trace_runner!(run_tokio_trace, reactor_tokio_event);

#[tokio::test]
async fn core_and_tokio_control_match_poll_order_results_and_wake_registration() {
    let expected = (
        vec![2, 1],
        vec![
            TracePoll::Pending(1),
            TracePoll::Ready(2),
            TracePoll::Ready(1),
        ],
        1,
    );
    assert_eq!(run_core_trace().await, expected);
    assert_eq!(run_tokio_trace().await, expected);
}

macro_rules! define_equivalence_runner {
    ($name:ident, $reactor:ident) => {
        async fn $name() -> (Vec<u8>, usize) {
            struct Sources {
                stream: FixedQueue<u8, 8>,
                input: FixedQueue<u8, 2>,
            }
            let mut sources = Sources {
                stream: FixedQueue::new(),
                input: FixedQueue::new(),
            };
            for value in 1..=3 {
                sources.stream.push(value).unwrap();
            }
            sources.input.push(9).unwrap();
            let mut order = Vec::new();
            let mut after = 0;

            let result: Result<(), Infallible> = kittens::__private::$reactor! {
                policy {
                    selection: biased;
                    required_phases: [after_event];
                }

                #[source(stream)]
                #[readiness(may_remain_ready)]
                #[yields_to(input, when = buffered)]
                #[drain(max = 2)]
                item = sources.stream => {
                    order.push(item);
                    if item == 3 {
                        Ok(Control::Stop(()))
                    } else {
                        Ok(Control::Continue)
                    }
                }

                #[source(input)]
                #[readiness(may_remain_ready)]
                item = sources.input => {
                    order.push(item);
                    Ok(Control::Continue)
                }

                after_event {
                    after += 1;
                    Ok(())
                }
            };
            result.unwrap();
            (order, after)
        }
    };
}

define_equivalence_runner!(run_core_event, reactor_event);
define_equivalence_runner!(run_core_slots, reactor_slots);
define_equivalence_runner!(run_tokio_event, reactor_tokio_event);
define_equivalence_runner!(run_tokio_slots, reactor_tokio_slots);

#[test]
fn retained_expansion_future_sizes_are_recorded() {
    let core_event = run_core_event();
    let core_slots = run_core_slots();
    let tokio_event = run_tokio_event();
    let tokio_slots = run_tokio_slots();
    eprintln!(
        "equivalence future sizes: core-event={} core-slots={} tokio-event={} tokio-slots={}",
        std::mem::size_of_val(&core_event),
        std::mem::size_of_val(&core_slots),
        std::mem::size_of_val(&tokio_event),
        std::mem::size_of_val(&tokio_slots),
    );
}

#[tokio::test]
async fn all_four_retained_expansions_match_on_the_scripted_oracle() {
    let expected = (vec![9, 1, 2, 3], 2);
    assert_eq!(run_core_event().await, expected);
    assert_eq!(run_core_slots().await, expected);
    assert_eq!(run_tokio_event().await, expected);
    assert_eq!(run_tokio_slots().await, expected);
}
