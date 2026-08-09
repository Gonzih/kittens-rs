//! K2R-0 touch-protocol oracles: the finding-12 interleavings (IRQ before
//! registration, during read, after flag sample, INT stuck, read failure,
//! startup with INT asserted, generation wrap) plus edge-reconstruction and
//! honest-coalescing traces.

#![allow(missing_docs)]

use kittens_render::touch::{
    Activation, ContactEdge, TouchDelta, TouchGenerations, TouchPoint, TouchReader, TouchReport,
    TouchService,
};

fn point(id: u8, x: u16, y: u16) -> TouchPoint {
    TouchPoint { id, x, y }
}

fn report(points: &[TouchPoint]) -> TouchReport {
    let mut slots = [None, None];
    for (slot, p) in slots.iter_mut().zip(points.iter()) {
        *slot = Some(*p);
    }
    TouchReport { points: slots }
}

/// Scripted reader: a queue of snapshot results, an INT line script, and an
/// optional "produce during read" injection to reproduce the mid-read IRQ
/// deterministically.
struct ScriptedReader<'g> {
    snapshots: Vec<Result<TouchReport, ()>>,
    int_after: Vec<bool>,
    produce_during_read: Vec<bool>,
    generations: &'g TouchGenerations,
    reads: usize,
}

impl TouchReader for ScriptedReader<'_> {
    type Error = ();

    fn read_snapshot(&mut self) -> Result<TouchReport, ()> {
        let index = self.reads;
        self.reads += 1;
        if self
            .produce_during_read
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            // The controller fires again while our I2C read is in flight.
            self.generations.produce();
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
fn idle_activation_services_nothing() {
    let generations = TouchGenerations::new();
    let mut service = TouchService::new(2);
    let mut reader = ScriptedReader {
        snapshots: vec![],
        int_after: vec![false],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };
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
        "idle→pending produce requests a wake"
    );

    let mut service = TouchService::new(2);
    let down = report(&[point(1, 100, 200)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Ok(down)],
        int_after: vec![false, false],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };

    let mut surfaced = Vec::new();
    let outcome = service.service(&generations, &mut reader, |r, d| surfaced.push((r, d)));
    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert!(!generations.is_pending(), "generation consumed");
    assert_eq!(surfaced.len(), 1);
    assert_eq!(
        surfaced[0].1.edges[0],
        Some(ContactEdge::Down(point(1, 100, 200))),
        "first contact reconstructs as Down"
    );
}

#[test]
fn produce_during_read_is_not_lost() {
    let generations = TouchGenerations::new();
    generations.produce();

    let mut service = TouchService::new(4);
    let first = report(&[point(1, 10, 10)]);
    let second = report(&[point(1, 20, 20)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Ok(first), Ok(second)],
        // INT deasserted after both reads.
        int_after: vec![false, false, false],
        // The controller produces again while the FIRST read is in flight.
        produce_during_read: vec![true, false],
        generations: &generations,
        reads: 0,
    };

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);
    assert_eq!(
        outcome,
        Activation::Idle { surfaced: 2 },
        "the mid-read generation drives a second service in the same activation"
    );
    assert_eq!(count, 2);
    assert!(!generations.is_pending());
}

#[test]
fn stuck_int_line_is_bounded_by_the_activation_budget() {
    let generations = TouchGenerations::new();
    generations.produce();

    let mut service = TouchService::new(2);
    let snap = report(&[point(1, 5, 5)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Ok(snap), Ok(snap), Ok(snap), Ok(snap)],
        // INT stays asserted forever: a stuck line must not monopolize.
        int_after: vec![true, true, true, true, true],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);
    assert_eq!(
        outcome,
        Activation::BudgetExhausted { surfaced: 2 },
        "budget bounds the activation; the caller re-arms"
    );
    assert_eq!(count, 2);
    assert_eq!(reader.reads, 2, "exactly budget reads, no more");
}

#[test]
fn read_failure_restores_pending_and_the_next_activation_recovers() {
    let generations = TouchGenerations::new();
    generations.produce();

    let mut service = TouchService::new(2);
    let snap = report(&[point(2, 30, 40)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Err(()), Ok(snap)],
        int_after: vec![false, false, false],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };

    let outcome = service.service(&generations, &mut reader, |_, _| {
        panic!("failed read surfaces nothing")
    });
    assert_eq!(outcome, Activation::ReadFailed { surfaced: 0 });
    assert!(
        generations.is_pending(),
        "the failed generation survives for the next activation"
    );

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);
    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(count, 1, "next activation services the restored generation");
}

#[test]
fn startup_with_int_already_asserted_services_without_a_produce() {
    let generations = TouchGenerations::new();
    // No produce() ever happened — the line was already low at boot.
    let mut service = TouchService::new(2);
    let snap = report(&[point(1, 1, 1)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Ok(snap)],
        int_after: vec![true, false],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };

    let mut count = 0;
    let outcome = service.service(&generations, &mut reader, |_, _| count += 1);
    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(count, 1, "asserted INT alone drives a service");
}

#[test]
fn generation_wrap_is_harmless_by_equality() {
    // Seed the pair two steps below the wrap boundary so the loop crosses
    // u32::MAX → 0 for real; equality (never ordering) makes it harmless.
    let generations = TouchGenerations::new_at(u32::MAX - 2);
    for _ in 0..8 {
        generations.produce();
        assert!(generations.is_pending());
        // Service everything pending in one snapshot.
        let mut service = TouchService::new(1);
        let mut reader = ScriptedReader {
            snapshots: vec![Ok(TouchReport::default())],
            int_after: vec![false, false],
            produce_during_read: vec![],
            generations: &generations,
            reads: 0,
        };
        let _ = service.service(&generations, &mut reader, |_, _| {});
        assert!(!generations.is_pending(), "equality, not ordering");
    }
}

#[test]
fn coalescing_is_honest_edges_between_surfaced_reports() {
    let generations = TouchGenerations::new();
    // Three produces, one read: the intermediate states coalesce into the
    // single surfaced snapshot — and the machine claims nothing else.
    generations.produce();
    generations.produce();
    generations.produce();

    let mut service = TouchService::new(2);
    let only = report(&[point(1, 50, 60)]);
    let mut reader = ScriptedReader {
        snapshots: vec![Ok(only)],
        int_after: vec![false, false],
        produce_during_read: vec![],
        generations: &generations,
        reads: 0,
    };

    let mut surfaced = Vec::new();
    let outcome = service.service(&generations, &mut reader, |r, d| surfaced.push((r, d)));
    assert_eq!(outcome, Activation::Idle { surfaced: 1 });
    assert_eq!(surfaced.len(), 1, "three produces, one complete report");
    assert_eq!(service.latest(), only);
}

#[test]
fn edge_reconstruction_covers_down_move_up_and_two_contacts() {
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
            to: point(1, 12, 12)
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
            to: point(2, 95, 95)
        })
    );

    let d4 = TouchDelta::between(&one_up, &empty);
    assert_eq!(d4.edges[0], Some(ContactEdge::Up(point(2, 95, 95))));
}
