//! K2R-0 state-table and coverage oracles for the witness-driven
//! demand/sweep machine (exit-review round 1 API: crate-owned `Sweep<S>`,
//! provenance-branded settlement, invalidation terminating the affected
//! epoch, abandon recovery, written-milestone vocabulary).
//!
//! Stripes are "written" here by running a real model transfer per stripe —
//! the only mint for a [`StripeWritten`] witness is a `Completed`
//! settlement, so these oracles exercise the full transfer→sweep→demand
//! composition (finding 4).

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use kittens_render::demand::{ForeignSweep, FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::{InvalidPlan, Sweep, SweepPlan};
use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

const PANEL: Region = Region {
    x: 0,
    y: 0,
    width: 8,
    height: 4,
};

fn plan() -> SweepPlan {
    SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(PANEL), 2).expect("valid plan")
}

fn demand() -> FrameDemand {
    FrameDemand::new(0, plan())
}

// --- minimal model transfer for witness minting -------------------------

struct Hw {
    done: Mutex<bool>,
    fail: Mutex<bool>,
}

struct ModelTransfer {
    hw: Arc<Hw>,
    resources: Option<((), ())>,
    settled: Option<TransferOutcome>,
}

impl OwnedTransfer for ModelTransfer {
    type Transport = ();
    type Buffer = ();

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_some() {
            return Poll::Ready(());
        }
        if *self.hw.done.lock().expect("hw") {
            self.settled = Some(if *self.hw.fail.lock().expect("hw") {
                TransferOutcome::Failed
            } else {
                TransferOutcome::Completed
            });
            return Poll::Ready(());
        }
        Poll::Pending
    }

    fn cancel(&mut self) {
        if self.settled.is_none() {
            self.settled = Some(TransferOutcome::Cancelled);
        }
    }

    fn recover(mut self) -> Recovered<(), ()> {
        let outcome = self.settled.expect("settled");
        let _ = self.resources.take();
        Recovered {
            transport: (),
            buffer: (),
            outcome,
        }
    }
}

/// Transfers one stripe of `sweep` through a model transfer with the given
/// outcome, returning whether a witness could be minted and marked.
fn transfer_next_stripe<S>(sweep: &mut Sweep<S>, outcome: TransferOutcome) -> bool {
    let target = sweep.next_target().expect("stripe remains");
    let hw = Arc::new(Hw {
        done: Mutex::new(false),
        fail: Mutex::new(false),
    });
    let transfer = ModelTransfer {
        hw: Arc::clone(&hw),
        resources: Some(((), ())),
        settled: None,
    };
    let mut flight = InFlight::new(transfer, (), target);
    match outcome {
        TransferOutcome::Completed => *hw.done.lock().expect("hw") = true,
        TransferOutcome::Failed => {
            *hw.done.lock().expect("hw") = true;
            *hw.fail.lock().expect("hw") = true;
        }
        TransferOutcome::Cancelled => flight.begin_drain(),
    }
    let waker = Waker::noop().clone();
    let mut cx = Context::from_waker(&waker);
    let mut settled = match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("model settles immediately"),
    };
    match settled.stripe_written() {
        Some(witness) => {
            sweep.mark_written(witness).expect("in-order witness");
            true
        }
        None => false,
    }
}

/// Fully writes a sweep through model transfers and finishes it.
fn write_all<S>(mut sweep: Sweep<S>) -> (kittens_render::sweep::SweepWritten, S) {
    while !sweep.is_complete() {
        assert!(transfer_next_stripe(&mut sweep, TransferOutcome::Completed));
    }
    sweep.finish().expect("fully covered")
}

// --- oracles ------------------------------------------------------------

#[test]
fn clean_demand_mints_nothing() {
    let mut demand = demand();
    assert!(demand.begin_sweep(Tick(0), ()).is_none());
}

#[test]
fn requests_coalesce_and_epochs_are_monotonic() {
    let mut demand = demand();
    demand.request();
    demand.request();

    let sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");
    assert!(!demand.is_dirty());
    assert!(demand.begin_sweep(Tick(0), ()).is_none(), "one in flight");
    assert_eq!(sweep.epoch().get(), 0);

    let (written, ()) = write_all(sweep);
    assert_eq!(
        demand.finish_written(written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );

    demand.request();
    let sweep = demand.begin_sweep(Tick(1), ()).expect("second");
    assert_eq!(sweep.epoch().get(), 1, "strictly monotonic");
    let (aborted, ()) = sweep.abort();
    demand.finish_failed(aborted, Tick(1)).expect("active");
}

#[test]
fn cancelled_and_failed_transfers_cannot_mark_coverage() {
    let mut demand = demand();
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    assert!(
        !transfer_next_stripe(&mut sweep, TransferOutcome::Cancelled),
        "a cancelled transfer mints no witness"
    );
    assert!(
        !transfer_next_stripe(&mut sweep, TransferOutcome::Failed),
        "a failed transfer mints no witness"
    );
    assert!(!sweep.is_complete());
    // No caller assertion can complete an uncovered sweep.
    let Err(sweep) = sweep.finish() else {
        panic!("uncovered sweep must not finish")
    };
    // The only paths out are more Completed transfers or abort.
    let (aborted, ()) = sweep.abort();
    demand.finish_failed(aborted, Tick(1)).expect("active");
    assert!(demand.is_dirty());
    assert!(demand.full_repaint_required());
}

#[test]
fn snapshot_is_immutable_through_the_sweep_and_returned_at_the_end() {
    let mut demand = demand();
    demand.request();
    let sweep = demand
        .begin_sweep(Tick(0), alloc_free_scene(7))
        .expect("sweep");
    assert_eq!(*sweep.snapshot(), 7, "shared access only");
    let (written, scene) = write_all(sweep);
    assert_eq!(scene, 7, "snapshot returned at settlement");
    demand.finish_written(written, Tick(1)).expect("active");
}

const fn alloc_free_scene(value: u32) -> u32 {
    value
}

#[test]
fn foreign_and_stale_settlement_is_rejected_without_mutation() {
    let mut left = demand();
    let mut right = demand();
    left.request();
    right.request();

    let left_sweep = left.begin_sweep(Tick(0), ()).expect("left sweep");
    let right_sweep = right.begin_sweep(Tick(0), ()).expect("right sweep");
    let (left_written, ()) = write_all(left_sweep);
    let (right_written, ()) = write_all(right_sweep);

    // Both minted epoch 0 — provenance branding, not epoch numbers, must
    // reject the swap (finding 6), in release builds, without mutation.
    assert_eq!(
        right.finish_written(left_written, Tick(1)),
        Err(ForeignSweep)
    );
    assert!(right.sweeping().is_some(), "no mutation on rejection");

    // The rightful witnesses settle normally afterwards.
    assert_eq!(
        right.finish_written(right_written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    // left's witness was consumed by the failed attempt on `right`;
    // recover left via abandon (its sweep value is gone).
    left.abandon_active();
    assert!(left.is_dirty());
}

#[test]
fn invalidation_mid_sweep_discards_that_sweeps_settlement() {
    let mut demand = demand();
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    demand.invalidate(); // transport reset mid-sweep

    let (written, ()) = write_all(sweep);
    assert_eq!(
        demand.finish_written(written, Tick(5)),
        Ok(WrittenDisposition::DiscardedByInvalidation),
        "the suspect sweep is discarded, not trusted"
    );
    assert!(demand.is_dirty(), "demand retained");
    assert!(demand.full_repaint_required(), "obligation retained");

    // Throttle did NOT advance to Tick(5): with min_interval 0 this is
    // observable via a nonzero-interval machine.
    let mut throttled = FrameDemand::new(10, plan());
    throttled.request();
    let sweep = throttled.begin_sweep(Tick(0), ()).expect("sweep");
    throttled.invalidate();
    let (written, ()) = write_all(sweep);
    assert_eq!(
        throttled.finish_written(written, Tick(5)),
        Ok(WrittenDisposition::DiscardedByInvalidation)
    );
    assert!(
        throttled.begin_sweep(Tick(5), ()).is_some(),
        "discarded settlement did not advance the throttle"
    );
}

#[test]
fn effective_settlement_advances_throttle_and_clears_obligation() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("first");
    let (written, ()) = write_all(sweep);
    assert_eq!(
        demand.finish_written(written, Tick(0)),
        Ok(WrittenDisposition::Effective)
    );
    assert!(!demand.full_repaint_required());

    demand.request();
    assert!(demand.begin_sweep(Tick(5), ()).is_none(), "throttled");
    assert_eq!(demand.eligible_at(), Some(Tick(10)));
    assert!(demand.begin_sweep(Tick(10), ()).is_some(), "eligible at 10");
}

#[test]
fn failed_sweep_retains_demand_and_throttle_position() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("first");
    let (written, ()) = write_all(sweep);
    demand.finish_written(written, Tick(0)).expect("active");

    demand.request();
    let sweep = demand.begin_sweep(Tick(10), ()).expect("second");
    let (aborted, ()) = sweep.abort();
    demand.finish_failed(aborted, Tick(11)).expect("active");

    assert!(demand.is_dirty());
    assert!(demand.full_repaint_required());
    assert!(
        demand.begin_sweep(Tick(11), ()).is_some(),
        "failure did not advance the throttle beyond last_written(0)+10"
    );
}

#[test]
fn abandon_recovers_a_dropped_sweep() {
    let mut demand = demand();
    demand.request();
    {
        let _dropped = demand.begin_sweep(Tick(0), ()).expect("sweep");
        // Early return / panic path: the sweep value is dropped.
    }
    assert!(
        demand.sweeping().is_some(),
        "machine believes a sweep is active"
    );
    assert!(
        demand.begin_sweep(Tick(0), ()).is_none(),
        "wedged without recovery"
    );

    demand.abandon_active();
    assert!(demand.sweeping().is_none());
    assert!(demand.is_dirty(), "demand retained");
    assert!(demand.full_repaint_required());
    assert!(demand.begin_sweep(Tick(0), ()).is_some(), "recovered");

    // Idempotent when idle.
    demand.abandon_active();
}

#[test]
fn plan_tiles_the_panel_exactly_including_partial_last_stripe() {
    let board = Region {
        x: 0,
        y: 0,
        width: 368,
        height: 448,
    };
    let exact =
        SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(board), 16).expect("valid");
    assert_eq!(exact.stripe_count(), 28);

    let uneven =
        SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(board), 30).expect("valid");
    assert_eq!(uneven.stripe_count(), 15);
    let mut covered = 0u32;
    let mut expected_y = 0;
    for index in 0..uneven.stripe_count() {
        let region = uneven.region_at(index).expect("in range");
        assert_eq!(region.y, expected_y, "contiguous");
        assert_eq!(region.width, board.width);
        expected_y += region.height;
        covered += u32::from(region.height);
    }
    assert_eq!(covered, u32::from(board.height), "exact coverage");
    assert!(uneven.region_at(15).is_none());
}

#[test]
fn out_of_order_witnesses_are_rejected() {
    let mut demand = demand();
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    // Mint a duplicate target for stripe 0 BEFORE writing it (the only
    // legal way to obtain a stale-position target), write stripe 0
    // properly, then complete the duplicate: its witness is out of order
    // and rejected — replay through a real transfer cannot double-count.
    let duplicate = sweep.next_target().expect("stripe 0 duplicate");
    assert!(transfer_next_stripe(&mut sweep, TransferOutcome::Completed));

    let hw = Arc::new(Hw {
        done: Mutex::new(true),
        fail: Mutex::new(false),
    });
    let mut replay = InFlight::new(
        ModelTransfer {
            hw,
            resources: Some(((), ())),
            settled: None,
        },
        (),
        duplicate,
    );
    let waker = Waker::noop().clone();
    let mut cx = Context::from_waker(&waker);
    let mut settled = match replay.poll_complete(&mut cx) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("model settles"),
    };
    let witness = settled.stripe_written().expect("completed mints");
    let error = sweep.mark_written(witness).expect_err("out of order");
    assert_eq!(error.expected, plan().region_at(1));

    let (aborted, ()) = sweep.abort();
    demand.finish_failed(aborted, Tick(1)).expect("active");
}

#[test]
fn invalid_plans_are_rejected_including_overflow() {
    let custom = PanelGeometry::custom_unvalidated_panel;
    assert_eq!(
        SweepPlan::for_panel(
            custom(Region {
                x: 0,
                y: 0,
                width: 0,
                height: 4
            }),
            2
        )
        .unwrap_err(),
        InvalidPlan::EmptyPanel
    );
    assert_eq!(
        SweepPlan::for_panel(custom(PANEL), 0).unwrap_err(),
        InvalidPlan::ZeroStripe
    );
    assert_eq!(
        SweepPlan::for_panel(
            custom(Region {
                x: 0,
                y: u16::MAX,
                width: 8,
                height: 2
            }),
            2
        )
        .unwrap_err(),
        InvalidPlan::Overflow,
        "y + height overflow rejected"
    );
}

#[test]
fn request_during_active_sweep_survives_settlement() {
    let mut demand = demand();
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    demand.request(); // arrives while the sweep is active
    assert!(demand.is_dirty(), "mid-sweep demand recorded");

    let (written, ()) = write_all(sweep);
    demand.finish_written(written, Tick(1)).expect("active");
    assert!(demand.is_dirty(), "mid-sweep demand survives settlement");
    let sweep = demand.begin_sweep(Tick(1), ()).expect("next epoch");
    assert_eq!(sweep.epoch().get(), 1);
    let (aborted, ()) = sweep.abort();
    demand.finish_failed(aborted, Tick(1)).expect("active");
}

#[test]
fn slow_sweep_throttles_from_its_finish_instant() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");
    let (written, ()) = write_all(sweep);
    // The sweep itself took 50 ticks; the throttle anchors at the finish
    // instant, not the begin instant.
    demand.finish_written(written, Tick(50)).expect("active");

    demand.request();
    assert!(demand.begin_sweep(Tick(55), ()).is_none(), "inside window");
    assert_eq!(demand.eligible_at(), Some(Tick(60)));
    assert!(demand.begin_sweep(Tick(60), ()).is_some());
}

#[test]
fn regressing_finish_time_is_clamped() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("first");
    let (written, ()) = write_all(sweep);
    demand.finish_written(written, Tick(20)).expect("active");

    demand.request();
    let sweep = demand.begin_sweep(Tick(30), ()).expect("second");
    let (written, ()) = write_all(sweep);
    // A regressing platform clock reports Tick(5): the throttle position
    // must not move backward.
    demand.finish_written(written, Tick(5)).expect("active");

    demand.request();
    assert!(
        demand.begin_sweep(Tick(25), ()).is_none(),
        "throttle still anchored at Tick(20), not the regressed Tick(5)"
    );
    assert!(demand.begin_sweep(Tick(30), ()).is_some());
}

#[test]
fn abandoned_epochs_witnesses_are_terminally_rejected() {
    let mut demand = demand();
    demand.request();
    let old_sweep = demand.begin_sweep(Tick(0), ()).expect("epoch 0");
    demand.abandon_active();

    let new_sweep = demand.begin_sweep(Tick(0), ()).expect("epoch 1");
    assert_eq!(new_sweep.epoch().get(), 1);

    // The old sweep is still live and can still produce a witness — the
    // documented caller obligation is to drain it; the machine's guarantee
    // is that its witness is terminally rejected without mutation.
    let (old_written, ()) = write_all(old_sweep);
    assert_eq!(
        demand.finish_written(old_written, Tick(1)),
        Err(ForeignSweep)
    );
    assert_eq!(
        demand.sweeping(),
        Some(new_sweep.epoch()),
        "rejection did not disturb the live sweep"
    );

    let (written, ()) = write_all(new_sweep);
    demand.finish_written(written, Tick(1)).expect("active");
}
