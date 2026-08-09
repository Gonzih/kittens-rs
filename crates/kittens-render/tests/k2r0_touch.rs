//! K2R-0 touch-protocol oracles for exit-review findings 10–13: the
//! increment-then-latch wake handoff and its broken negative control;
//! persistent retry across INT-only work, read failure, budget exhaustion,
//! and exact generation alias; unchanged-snapshot edge honesty; a nonzero
//! service budget; plus the baseline latest-state/coalescing traces.

#![allow(missing_docs)]

use core::num::NonZeroU8;
use core::sync::atomic::{AtomicU32, Ordering};

use kittens_render::touch::{
    Activation, ContactEdge, TouchDelta, TouchGenerations, TouchPoint, TouchReader, TouchReport,
    TouchService,
};

fn budget(value: u8) -> NonZeroU8 {
    NonZeroU8::new(value).expect("test budgets are nonzero")
}

fn point(id: u8, x: u16, y: u16) -> TouchPoint {
    TouchPoint { id, x, y }
}

fn report(points: &[TouchPoint]) -> TouchReport {
    let mut slots = [None, None];
    for (slot, point) in slots.iter_mut().zip(points.iter()) {
        *slot = Some(*point);
    }
    TouchReport { points: slots }
}

/// Scripted reader: a queue of snapshot results, an INT-line script, and
/// optional produces injected during reads. `produce_wakes` records the
/// producer return value so wake handoffs are independently observable.
struct ScriptedReader<'g> {
    snapshots: Vec<Result<TouchReport, ()>>,
    int_after: Vec<bool>,
    produce_during_read: Vec<bool>,
    produce_wakes: Vec<bool>,
    generations: &'g TouchGenerations,
    reads: usize,
}

impl<'g> ScriptedReader<'g> {
    fn new(
        generations: &'g TouchGenerations,
        snapshots: Vec<Result<TouchReport, ()>>,
        int_after: Vec<bool>,
    ) -> Self {
        Self {
            snapshots,
            int_after,
            produce_during_read: Vec::new(),
            produce_wakes: Vec::new(),
            generations,
            reads: 0,
        }
    }

    fn with_produces_during_reads(mut self, script: Vec<bool>) -> Self {
        self.produce_during_read = script;
        self
    }
}

impl TouchReader for ScriptedReader<'_> {
    type Error = ();

    fn read_snapshot(&mut self) -> Result<TouchReport, Self::Error> {
        let index = self.reads;
        self.reads += 1;
        if self
            .produce_during_read
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            // The controller fires again while the contiguous I2C read is
            // in flight. The service has already claimed/cleared the latch.
            self.produce_wakes.push(self.generations.produce());
        }
        self.snapshots
            .get(index)
            .copied()
            .unwrap_or(Ok(TouchReport::default()))
    }

    fn int_asserted(&self) -> bool {
        self.int_after.get(self.reads).copied().unwrap_or(false)
    }
}

#[test]
fn service_budget_is_nonzero_by_construction() {
    let constructor: fn(NonZeroU8) -> TouchService = TouchService::new;
    let _service = constructor(budget(1));
    assert!(
        NonZeroU8::new(0).is_none(),
        "zero cannot be passed to the constructor"
    );
}

#[test]
fn idle_activation_services_nothing() {
    let generations = TouchGenerations::new();
    let mut service = TouchService::new(budget(2));
    let mut reader = ScriptedReader::new(&generations, vec![], vec![false]);

    let outcome = service.service(&generations, &mut reader, |_, _| {
        panic!("nothing to surface")
    });

    assert_eq!(outcome, Activation::Idle { surfaced: 0 });
    assert_eq!(reader.reads, 0, "no pending work, no I2C traffic");
}

#[test]
fn one_produce_surfaces_one_untorn_report_and_clears_pending() {
    let generations = TouchGenerations::new();
    assert!(
        generations.produce(),
        "idle-to-pending produce requests a wake"
    );

    let mut service = TouchService::new(budget(2));
    let down = report(&[point(1, 100, 200)]);
    let mut reader = ScriptedReader::new(&generations, vec![Ok(down)], vec![false, false]);
    let mut surfaced = Vec::new();

    let outcome = service.service(&generations, &mut reader, |snapshot, delta| {
        surfaced.push((snapshot, delta));
    });

    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert!(!generations.is_pending(), "successful handoff cleared");
    assert_eq!(surfaced.len(), 1);
    assert_eq!(
        surfaced[0].1.edges[0],
        Some(ContactEdge::Down(point(1, 100, 200))),
        "first contact reconstructs as Down"
    );
}

#[test]
fn produces_coalesce_while_pending_without_duplicate_wakes() {
    let generations = TouchGenerations::new();
    assert!(generations.produce(), "first produce wakes");
    assert!(!generations.produce(), "latched produce deduplicates");

    let mut service = TouchService::new(budget(2));
    let snapshot = report(&[point(1, 10, 10)]);
    let mut reader = ScriptedReader::new(&generations, vec![Ok(snapshot)], vec![false, false]);

    let outcome = service.service(&generations, &mut reader, |_, _| {});

    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(reader.reads, 1, "both produces coalesce into latest state");
    assert!(!generations.is_pending());
}

#[test]
fn produce_during_read_relatches_even_when_generation_wrap_is_not_needed() {
    let generations = TouchGenerations::new();
    assert!(generations.produce());

    let mut service = TouchService::new(budget(4));
    let first = report(&[point(1, 10, 10)]);
    let second = report(&[point(1, 20, 20)]);
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Ok(first), Ok(second)],
        vec![false, false, false],
    )
    .with_produces_during_reads(vec![true, false]);

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);

    assert_eq!(
        outcome,
        Activation::Idle { surfaced: 2 },
        "the cleared latch drives a second snapshot in the same activation"
    );
    assert_eq!(count, 2);
    assert_eq!(
        reader.produce_wakes,
        vec![true],
        "an arrival after the consumer claim owns the wake transition"
    );
    assert!(!generations.is_pending());
}

#[test]
fn produce_after_idle_handoff_requests_a_wake() {
    let generations = TouchGenerations::new();
    assert!(generations.produce());
    let mut service = TouchService::new(budget(1));
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Ok(TouchReport::default())],
        vec![false, false],
    );

    assert_eq!(
        service.service(&generations, &mut reader, |_, _| {}),
        Activation::Idle { surfaced: 1 }
    );
    assert!(!generations.is_pending(), "consumer completed the handoff");

    assert!(
        generations.produce(),
        "a producer after the idle handoff observes the clear and wakes"
    );
    assert!(generations.is_pending());
}

/// Deliberately preserves finding 10's removed check-before-increment
/// implementation, split into phases so its failing interleaving is exact.
struct BrokenWakeDedup {
    produced: AtomicU32,
    serviced: AtomicU32,
}

impl BrokenWakeDedup {
    fn with_one_pending() -> Self {
        Self {
            produced: AtomicU32::new(1),
            serviced: AtomicU32::new(0),
        }
    }

    fn is_pending(&self) -> bool {
        self.produced.load(Ordering::SeqCst) != self.serviced.load(Ordering::SeqCst)
    }

    fn observe_pending_before_increment(&self) -> bool {
        self.is_pending()
    }

    fn service_and_exit_idle(&self) {
        let produced = self.produced.load(Ordering::SeqCst);
        self.serviced.store(produced, Ordering::SeqCst);
        assert!(
            !self.is_pending(),
            "broken consumer now believes it is idle"
        );
    }

    fn finish_produce_with_stale_observation(&self, was_pending: bool) -> bool {
        self.produced.fetch_add(1, Ordering::SeqCst);
        !was_pending
    }
}

#[test]
fn negative_control_check_before_increment_loses_idle_wake() {
    let broken = BrokenWakeDedup::with_one_pending();

    // Exact reviewed schedule: producer samples pending and pauses; service
    // consumes generation 1 and exits idle; producer increments to 2, then
    // suppresses the wake using its stale sample.
    let stale_was_pending = broken.observe_pending_before_increment();
    assert!(stale_was_pending);
    broken.service_and_exit_idle();
    let consumer_exited_idle = true;
    let wake_requested = broken.finish_produce_with_stale_observation(stale_was_pending);
    let work_pending = broken.is_pending();

    assert!(work_pending, "generation 2 is outstanding");
    assert!(!wake_requested, "the broken producer suppressed its wake");
    let progress_is_guaranteed = !work_pending || wake_requested || !consumer_exited_idle;
    assert!(
        !progress_is_guaranteed,
        "the oracle catches pending work with no active consumer or wake"
    );
}

#[test]
fn read_failure_keeps_retry_latched_and_next_activation_recovers() {
    let generations = TouchGenerations::new();
    assert!(generations.produce());

    let mut service = TouchService::new(budget(2));
    let snapshot = report(&[point(2, 30, 40)]);
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Err(()), Ok(snapshot)],
        vec![false, false, false],
    );

    let outcome = service.service(&generations, &mut reader, |_, _| {
        panic!("failed read surfaces nothing")
    });
    assert_eq!(outcome, Activation::ReadFailed { surfaced: 0 });
    assert!(generations.is_pending(), "failure retained retry authority");

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);
    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(count, 1, "the next activation services the retry latch");
    assert!(!generations.is_pending());
}

#[test]
fn startup_int_read_failure_retries_after_int_deasserts() {
    let generations = TouchGenerations::new();
    // There is no produce: startup INT alone creates the retry obligation.
    let mut service = TouchService::new(budget(2));
    let snapshot = report(&[point(1, 1, 1)]);
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Err(()), Ok(snapshot)],
        // Asserted before the failed read, deasserted on every later sample.
        vec![true, false, false],
    );

    assert_eq!(
        service.service(&generations, &mut reader, |_, _| {
            panic!("failed startup read surfaces nothing")
        }),
        Activation::ReadFailed { surfaced: 0 }
    );
    assert!(
        generations.is_pending(),
        "INT-only failure persists after the line deasserts"
    );

    let mut surfaced = 0;
    assert_eq!(
        service.service(&generations, &mut reader, |_, _| surfaced += 1),
        Activation::Idle { surfaced: 1 }
    );
    assert_eq!(surfaced, 1, "next activation retries without INT");
    assert!(!generations.is_pending());
}

#[test]
fn startup_with_int_already_asserted_services_without_a_produce() {
    let generations = TouchGenerations::new();
    let mut service = TouchService::new(budget(2));
    let snapshot = report(&[point(1, 1, 1)]);
    let mut reader = ScriptedReader::new(&generations, vec![Ok(snapshot)], vec![true, false]);

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);

    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(count, 1, "asserted INT alone drives a service");
}

#[test]
fn budget_exhaustion_keeps_retry_latched_after_int_deasserts() {
    let generations = TouchGenerations::new();
    let mut service = TouchService::new(budget(1));
    let snapshot = report(&[point(1, 5, 5)]);
    let mut stuck_reader = ScriptedReader::new(&generations, vec![Ok(snapshot)], vec![true, true]);

    assert_eq!(
        service.service(&generations, &mut stuck_reader, |_, _| {}),
        Activation::BudgetExhausted { surfaced: 1 }
    );
    assert_eq!(stuck_reader.reads, 1, "exactly the configured budget");
    assert!(
        generations.is_pending(),
        "budget exit retains authoritative retry"
    );

    // A later activation must honor the latch even though INT has now
    // deasserted and there is no new producer edge to wake it.
    let mut deasserted_reader =
        ScriptedReader::new(&generations, vec![Ok(snapshot)], vec![false, false]);
    assert_eq!(
        service.service(&generations, &mut deasserted_reader, |_, _| {}),
        Activation::Idle { surfaced: 1 }
    );
    assert_eq!(deasserted_reader.reads, 1);
    assert!(!generations.is_pending());
}

#[test]
fn stuck_int_identical_snapshots_emit_no_false_movement_edges() {
    let generations = TouchGenerations::new();
    let mut service = TouchService::new(budget(2));
    let identical = report(&[point(1, 5, 5)]);
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Ok(identical), Ok(identical)],
        vec![true, true, true],
    );
    let mut deltas = Vec::new();

    let outcome = service.service(&generations, &mut reader, |_, delta| deltas.push(delta));

    assert_eq!(outcome, Activation::BudgetExhausted { surfaced: 2 });
    assert_eq!(reader.reads, 2, "stuck INT is still budget-bounded");
    assert_eq!(deltas[0].edges[0], Some(ContactEdge::Down(point(1, 5, 5))));
    assert_eq!(
        deltas[1],
        TouchDelta::default(),
        "an identical complete TouchPoint emits no Moved edge"
    );
}

#[test]
fn generation_wrap_boundary_remains_serviceable() {
    let generations = TouchGenerations::new_at(u32::MAX - 2);
    for _ in 0..8 {
        assert!(generations.produce(), "idle transition wakes across wrap");
        assert!(generations.is_pending());

        let mut service = TouchService::new(budget(1));
        let mut reader = ScriptedReader::new(
            &generations,
            vec![Ok(TouchReport::default())],
            vec![false, false],
        );
        assert_eq!(
            service.service(&generations, &mut reader, |_, _| {}),
            Activation::Idle { surfaced: 1 }
        );
        assert!(!generations.is_pending());
    }
}

#[test]
fn seeded_two_to_the_32_produces_cannot_alias_pending_to_idle() {
    // Equal counters plus a set latch models 2^32 outstanding produces.
    // Equality-only logic returns idle here; the authoritative latch reads.
    let generations = TouchGenerations::new_pending_at(37);
    assert!(generations.is_pending(), "counter alias is still pending");

    let mut service = TouchService::new(budget(1));
    let mut reader = ScriptedReader::new(
        &generations,
        vec![Ok(TouchReport::default())],
        vec![false, false],
    );

    assert_eq!(
        service.service(&generations, &mut reader, |_, _| {}),
        Activation::Idle { surfaced: 1 },
        "aliased outstanding work receives a snapshot before idle"
    );
    assert_eq!(reader.reads, 1);
    assert!(
        !generations.is_pending(),
        "successful snapshot clears alias"
    );
}

#[test]
fn coalescing_is_honest_edges_between_surfaced_reports() {
    let generations = TouchGenerations::new();
    assert!(generations.produce());
    assert!(!generations.produce());
    assert!(!generations.produce());

    let mut service = TouchService::new(budget(2));
    let only = report(&[point(1, 50, 60)]);
    let mut reader = ScriptedReader::new(&generations, vec![Ok(only)], vec![false, false]);
    let mut surfaced = Vec::new();

    let outcome = service.service(&generations, &mut reader, |snapshot, delta| {
        surfaced.push((snapshot, delta));
    });

    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(surfaced.len(), 1, "three produces, one latest report");
    assert_eq!(service.latest(), only);
}

#[test]
fn edge_reconstruction_covers_down_changed_move_up_and_two_contacts() {
    let empty = TouchReport::default();
    let one_down = report(&[point(1, 10, 10)]);
    let both_down = report(&[point(1, 12, 12), point(2, 90, 90)]);
    let one_up = report(&[point(2, 95, 95)]);

    let d1 = TouchDelta::between(&empty, &one_down);
    assert_eq!(d1.edges[0], Some(ContactEdge::Down(point(1, 10, 10))));
    assert_eq!(d1.edges[1], None);

    let d2 = TouchDelta::between(&one_down, &both_down);
    assert_eq!(
        d2.edges[0],
        Some(ContactEdge::Moved {
            from: point(1, 10, 10),
            to: point(1, 12, 12),
        })
    );
    assert_eq!(d2.edges[1], Some(ContactEdge::Down(point(2, 90, 90))));

    let d3 = TouchDelta::between(&both_down, &one_up);
    assert_eq!(
        d3.edges[0],
        Some(ContactEdge::Up(point(1, 12, 12))),
        "contact 1 lifted"
    );
    assert_eq!(
        d3.edges[1],
        Some(ContactEdge::Moved {
            from: point(2, 90, 90),
            to: point(2, 95, 95),
        })
    );

    let d4 = TouchDelta::between(&one_up, &empty);
    assert_eq!(d4.edges[0], Some(ContactEdge::Up(point(2, 95, 95))));
}
