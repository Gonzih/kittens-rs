//! K2R-0 state-table and coverage oracles for the witness-driven
//! demand/sweep machine (exit-review round 1 API: crate-owned `Sweep<S>`,
//! provenance-branded settlement, invalidation terminating the affected
//! epoch, abandon recovery, written-milestone vocabulary).
//!
//! Stripes are settled here by running a real model transfer per stripe.
//! Target-driven start and cooperative delivery of the move-only written-or-
//! unwritten settlement exercise the full transfer→sweep→demand composition.

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use kittens_render::demand::{ForeignSweep, FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{FrameEpoch, PanelGeometry, Region};
use kittens_render::sweep::{InvalidPlan, StripeSettlement, StripeTarget, Sweep, SweepPlan};
use kittens_render::transfer::{
    FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome,
};

const PANEL: Region = Region {
    x: 0,
    y: 0,
    width: 8,
    height: 4,
};

fn relinquish_owned<T>(_value: T) {}

fn plan() -> SweepPlan {
    SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(PANEL), 2).expect("valid plan")
}

fn demand() -> FrameDemand {
    FrameDemand::new(0, plan())
}

fn observable_demand_state(demand: &FrameDemand) -> (bool, Option<FrameEpoch>, bool, Option<Tick>) {
    (
        demand.is_dirty(),
        demand.sweeping(),
        demand.full_repaint_required(),
        demand.eligible_at(),
    )
}

const ANCHOR_FINISH: Tick = Tick(20);
const ANCHOR_ELIGIBLE: Tick = Tick(30);
const BEFORE_ANCHOR_ELIGIBLE: Tick = Tick(29);
const REJECTION_TIME: Tick = Tick(100);

/// Establishes a real, non-`None` throttle anchor through a successful write,
/// proves its exact deadline, and returns the next active sweep.
fn begin_after_established_write(demand: &mut FrameDemand) -> Sweep<()> {
    demand.request();
    let anchor = demand.begin_sweep(Tick(0), ()).expect("anchor sweep");
    assert_eq!(anchor.epoch().get(), 0);
    let (written, ()) = write_all(anchor);
    assert_eq!(
        demand.finish_written(written, ANCHOR_FINISH),
        Ok(WrittenDisposition::Effective)
    );

    demand.request();
    assert!(
        demand.begin_sweep(BEFORE_ANCHOR_ELIGIBLE, ()).is_none(),
        "the established write installs a real throttle anchor"
    );
    assert_eq!(
        demand.eligible_at(),
        Some(ANCHOR_ELIGIBLE),
        "anchor eligibility is exact"
    );
    let active = demand
        .begin_sweep(ANCHOR_ELIGIBLE, ())
        .expect("active sweep at the exact anchor deadline");
    assert_eq!(active.epoch().get(), 1);
    active
}

/// Completes the rightful active epoch through failure, then proves a prior
/// rejected terminal witness changed neither the established non-`None`
/// throttle anchor nor the hidden next-epoch sequence. Querying the boundary
/// tick deliberately isolates stored policy state from wall-clock progression.
fn assert_rejection_preserved_future(
    demand: &mut FrameDemand,
    active: Sweep<()>,
    expected_successor: u64,
) {
    let (aborted, ()) = active
        .abort()
        .expect("rightful sweep has no outstanding target");
    demand
        .finish_failed(aborted, REJECTION_TIME)
        .expect("rightful abort remains accepted");
    assert!(
        demand.begin_sweep(BEFORE_ANCHOR_ELIGIBLE, ()).is_none(),
        "rejection preserved the established throttle anchor"
    );
    assert_eq!(
        demand.eligible_at(),
        Some(ANCHOR_ELIGIBLE),
        "future eligibility remains exactly last_written + min_interval"
    );
    let successor = demand
        .begin_sweep(ANCHOR_ELIGIBLE, ())
        .expect("successor remains eligible at the exact original deadline");
    assert_eq!(
        successor.epoch().get(),
        expected_successor,
        "rejection did not skip or reuse a hidden epoch"
    );
    let (aborted, ()) = successor
        .abort()
        .expect("successor has no outstanding target");
    demand
        .finish_failed(aborted, ANCHOR_ELIGIBLE)
        .expect("successor cleanup");
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

struct ModelStart {
    hw: Arc<Hw>,
    expected_region: Region,
}

impl FlightStarter for ModelStart {
    type Transfer = ModelTransfer;
    type Error = core::convert::Infallible;

    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        assert_eq!(
            region, self.expected_region,
            "target supplies the start region"
        );
        Ok(ModelTransfer {
            hw: self.hw,
            resources: Some(((), ())),
            settled: None,
        })
    }
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

impl Drop for ModelTransfer {
    fn drop(&mut self) {
        if self.resources.is_some() {
            self.cancel();
        }
    }
}

/// Drives one already-issued target to the requested model outcome and
/// returns its move-only settlement witness for cooperative owner delivery.
fn transfer_target(target: StripeTarget, outcome: TransferOutcome) -> StripeSettlement {
    let expected_region = target.region();
    let hw = Arc::new(Hw {
        done: Mutex::new(false),
        fail: Mutex::new(false),
    });
    let mut flight = target
        .start_flight(
            (),
            ModelStart {
                hw: Arc::clone(&hw),
                expected_region,
            },
        )
        .expect("infallible model start");
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
    let settled = match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("model settles immediately"),
    };
    assert_eq!(settled.region(), expected_region);
    let ((), (), (), settlement) = settled.into_parts();
    assert_eq!(settlement.outcome(), outcome);
    settlement
}

/// Issues, transfers, and reconciles the next stripe.
fn transfer_next_stripe<S>(sweep: &mut Sweep<S>, outcome: TransferOutcome) -> TransferOutcome {
    let target = sweep.next_target().expect("stripe remains");
    let settlement = transfer_target(target, outcome);
    sweep.settle(settlement).expect("matching settlement")
}

/// Fully writes a sweep through model transfers and finishes it.
fn write_all<S>(mut sweep: Sweep<S>) -> (kittens_render::sweep::SweepWritten, S) {
    while !sweep.is_complete() {
        assert_eq!(
            transfer_next_stripe(&mut sweep, TransferOutcome::Completed),
            TransferOutcome::Completed
        );
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
    let (aborted, ()) = sweep.abort().expect("new sweep has no outstanding target");
    demand.finish_failed(aborted, Tick(1)).expect("active");
}

fn assert_unwritten_settlement_poisons(outcome: TransferOutcome) {
    let mut demand = demand();
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    assert_eq!(transfer_next_stripe(&mut sweep, outcome), outcome);
    assert!(sweep.is_poisoned(), "unwritten settlement poisons");
    assert!(
        sweep.next_target().is_none(),
        "poisoned sweep cannot mint a retry target"
    );
    assert!(!sweep.is_complete());
    let before = (
        sweep.epoch(),
        sweep.next_region(),
        sweep.full_repaint(),
        sweep.is_complete(),
        sweep.is_poisoned(),
    );
    let Err(mut sweep) = sweep.finish() else {
        panic!("poisoned sweep must not finish")
    };
    assert_eq!(
        (
            sweep.epoch(),
            sweep.next_region(),
            sweep.full_repaint(),
            sweep.is_complete(),
            sweep.is_poisoned(),
        ),
        before,
        "failed finish returns the poisoned sweep unchanged"
    );
    assert!(sweep.next_target().is_none(), "only abort remains");
    let (aborted, ()) = sweep
        .abort()
        .expect("poisoned settlement cleared outstanding");
    demand.finish_failed(aborted, Tick(1)).expect("active");
    assert!(demand.is_dirty());
    assert!(demand.sweeping().is_none());
    assert!(demand.full_repaint_required());
}

#[test]
fn cancelled_settlement_poisons_sweep() {
    assert_unwritten_settlement_poisons(TransferOutcome::Cancelled);
}

#[test]
fn failed_settlement_poisons_sweep() {
    assert_unwritten_settlement_poisons(TransferOutcome::Failed);
}

#[test]
fn snapshot_is_owned_through_the_sweep_and_returned_at_the_end() {
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
    let mut left = FrameDemand::new(10, plan());
    let mut right = FrameDemand::new(10, plan());

    let left_sweep = begin_after_established_write(&mut left);
    let right_sweep = begin_after_established_write(&mut right);
    let (left_written, ()) = write_all(left_sweep);

    // Both minted epoch 1 — provenance branding, not epoch numbers, must
    // reject the swap (finding 6), in release builds, without mutation.
    let before = observable_demand_state(&right);
    assert_eq!(
        right.finish_written(left_written, REJECTION_TIME),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&right),
        before,
        "every observable stays unchanged on foreign written rejection"
    );

    // left's witness was consumed by the failed attempt on `right`;
    // recover left via abandon (its sweep value is gone).
    left.abandon_active();
    assert!(left.is_dirty());
    assert_rejection_preserved_future(&mut right, right_sweep, 2);
}

#[test]
fn foreign_failed_settlement_is_rejected_without_mutation() {
    let mut left = FrameDemand::new(10, plan());
    let mut right = FrameDemand::new(10, plan());

    let left_sweep = begin_after_established_write(&mut left);
    let right_sweep = begin_after_established_write(&mut right);
    let (left_aborted, ()) = left_sweep
        .abort()
        .expect("left sweep has no outstanding target");

    let before = observable_demand_state(&right);
    assert_eq!(
        right.finish_failed(left_aborted, REJECTION_TIME),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&right),
        before,
        "foreign abort cannot disturb the active demand state"
    );
    left.abandon_active();
    assert_rejection_preserved_future(&mut right, right_sweep, 2);
}

#[test]
fn stale_failed_settlement_is_rejected_without_mutation() {
    let mut demand = FrameDemand::new(10, plan());
    let old = begin_after_established_write(&mut demand);
    let (old_aborted, ()) = old.abort().expect("old sweep has no outstanding target");
    demand.abandon_active();
    let replacement = demand
        .begin_sweep(ANCHOR_ELIGIBLE, ())
        .expect("replacement");
    assert_eq!(replacement.epoch().get(), 2);

    let before = observable_demand_state(&demand);
    assert_eq!(
        demand.finish_failed(old_aborted, REJECTION_TIME),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&demand),
        before,
        "stale abort rejection leaves all observable state unchanged"
    );
    assert_rejection_preserved_future(&mut demand, replacement, 3);
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

fn assert_idle_invalidation_discards_exactly_next(demand: &mut FrameDemand, expected_epoch: u64) {
    demand.invalidate();
    let replacement = demand
        .begin_sweep(Tick(50), ())
        .expect("sticky idle invalidation raises immediately eligible demand");
    assert_eq!(replacement.epoch().get(), expected_epoch);
    let (written, ()) = write_all(replacement);
    assert_eq!(
        demand.finish_written(written, Tick(50)),
        Ok(WrittenDisposition::DiscardedByInvalidation),
        "the next minted sweep inherits the idle invalidation"
    );
    assert!(demand.is_dirty());
    assert!(demand.full_repaint_required());

    let repaint = demand.begin_sweep(Tick(50), ()).expect(
        "discarded replacement did not advance throttle and consumed the pending latch once",
    );
    assert_eq!(repaint.epoch().get(), expected_epoch + 1);
    let (written, ()) = write_all(repaint);
    assert_eq!(
        demand.finish_written(written, Tick(51)),
        Ok(WrittenDisposition::Effective),
        "only the first replacement is marked non-clearing"
    );
    assert!(!demand.full_repaint_required());
}

#[test]
fn idle_invalidation_after_abort_sticks_to_the_replacement() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("initial sweep");
    let (aborted, ()) = sweep
        .abort()
        .expect("initial sweep has no outstanding target");
    demand
        .finish_failed(aborted, Tick(1))
        .expect("active abort");

    assert_idle_invalidation_discards_exactly_next(&mut demand, 1);
}

#[test]
fn idle_invalidation_after_abandon_sticks_to_the_replacement() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    {
        let _relinquished_sweep = demand.begin_sweep(Tick(0), ()).expect("initial sweep");
    }
    demand.abandon_active();

    assert_idle_invalidation_discards_exactly_next(&mut demand, 1);
}

#[test]
fn throttled_begin_does_not_consume_idle_invalidation() {
    let mut demand = FrameDemand::new(10, plan());
    demand.request();
    let first = demand.begin_sweep(Tick(0), ()).expect("initial sweep");
    let (written, ()) = write_all(first);
    assert_eq!(
        demand.finish_written(written, Tick(0)),
        Ok(WrittenDisposition::Effective)
    );

    demand.invalidate();
    assert!(
        demand.begin_sweep(Tick(9), ()).is_none(),
        "the forced repaint is still throttled by the prior effective write"
    );
    let replacement = demand
        .begin_sweep(Tick(10), ())
        .expect("first eligible mint still receives the pending invalidation");
    assert_eq!(replacement.epoch().get(), 1);
    let (written, ()) = write_all(replacement);
    assert_eq!(
        demand.finish_written(written, Tick(10)),
        Ok(WrittenDisposition::DiscardedByInvalidation)
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
    let (aborted, ()) = sweep
        .abort()
        .expect("failed sweep has no outstanding target");
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
fn only_one_target_is_outstanding_and_settlement_clears_it() {
    let mut demand = demand();
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");

    let target = sweep.next_target().expect("first target");
    let before = (
        sweep.epoch(),
        sweep.next_region(),
        sweep.full_repaint(),
        sweep.is_complete(),
        sweep.is_poisoned(),
    );
    assert!(
        sweep.next_target().is_none(),
        "a position cannot mint a duplicate target"
    );
    assert_eq!(
        (
            sweep.epoch(),
            sweep.next_region(),
            sweep.full_repaint(),
            sweep.is_complete(),
            sweep.is_poisoned(),
        ),
        before,
        "the refused duplicate mint changes no sweep state"
    );
    let Err(returned) = sweep.finish() else {
        panic!("an outstanding target prevents finish")
    };
    sweep = returned;
    assert_eq!(
        (
            sweep.epoch(),
            sweep.next_region(),
            sweep.full_repaint(),
            sweep.is_complete(),
            sweep.is_poisoned(),
        ),
        before,
        "outstanding-finish rejection returns the sweep unchanged"
    );
    let Err(returned) = sweep.abort() else {
        panic!("an outstanding target prevents abort")
    };
    sweep = returned;
    assert_eq!(
        (
            sweep.epoch(),
            sweep.next_region(),
            sweep.full_repaint(),
            sweep.is_complete(),
            sweep.is_poisoned(),
        ),
        before,
        "outstanding-abort rejection returns the sweep unchanged"
    );

    let settlement = transfer_target(target, TransferOutcome::Completed);
    assert_eq!(sweep.settle(settlement), Ok(TransferOutcome::Completed));
    assert_eq!(sweep.next_region(), plan().region_at(1));
    let incomplete = (
        sweep.epoch(),
        sweep.next_region(),
        sweep.full_repaint(),
        sweep.is_complete(),
        sweep.is_poisoned(),
    );
    let Err(returned) = sweep.finish() else {
        panic!("healthy but incomplete coverage prevents finish")
    };
    sweep = returned;
    assert_eq!(
        (
            sweep.epoch(),
            sweep.next_region(),
            sweep.full_repaint(),
            sweep.is_complete(),
            sweep.is_poisoned(),
        ),
        incomplete,
        "incomplete-finish rejection returns the sweep unchanged"
    );
    let second = sweep
        .next_target()
        .expect("settlement cleared outstanding for the next position");
    let Err(returned) = sweep.abort() else {
        panic!("the second outstanding target also prevents abort")
    };
    sweep = returned;
    let cancelled = transfer_target(second, TransferOutcome::Cancelled);
    assert_eq!(
        sweep.settle(cancelled),
        Ok(TransferOutcome::Cancelled),
        "begin_drain + poll_complete produces the settlement that clears outstanding"
    );
    assert!(sweep.is_poisoned());
    let (aborted, ()) = sweep
        .abort()
        .expect("settled cancellation permits shutdown abort");
    demand.finish_failed(aborted, Tick(1)).expect("active");
}

#[test]
fn cross_demand_stripe_settlement_is_rejected_without_mutation() {
    let mut left_demand = demand();
    let mut right_demand = demand();
    left_demand.request();
    right_demand.request();
    let mut left = left_demand.begin_sweep(Tick(0), ()).expect("left");
    let mut right = right_demand.begin_sweep(Tick(0), ()).expect("right");

    let left_target = left.next_target().expect("left target");
    let right_target = right.next_target().expect("right target");
    assert_eq!(left_target.region(), right_target.region());
    let foreign = transfer_target(left_target, TransferOutcome::Completed);
    let rightful = transfer_target(right_target, TransferOutcome::Completed);

    let before = (
        right.epoch(),
        right.next_region(),
        right.full_repaint(),
        right.is_complete(),
        right.is_poisoned(),
    );
    let error = right
        .settle(foreign)
        .expect_err("cross-demand written settlement is foreign");
    assert_eq!(error.expected, before.1);
    assert_eq!(
        (
            right.epoch(),
            right.next_region(),
            right.full_repaint(),
            right.is_complete(),
            right.is_poisoned(),
        ),
        before,
        "foreign stripe rejection preserves progress, poison, and target state"
    );
    assert!(
        right.next_target().is_none(),
        "the rightful target remains outstanding after rejection"
    );
    assert_eq!(
        right.settle(rightful),
        Ok(TransferOutcome::Completed),
        "the rightful settlement still advances"
    );

    // The left settlement witness was consumed by the deliberate foreign
    // reconciliation attempt, so its sweep remains outstanding and cannot
    // abort. No transfer remains live; use the explicit lost-token recovery.
    relinquish_owned(left);
    left_demand.abandon_active();
    let (right_written, ()) = write_all(right);
    right_demand
        .finish_written(right_written, Tick(1))
        .expect("right completes");
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
    let (aborted, ()) = sweep.abort().expect("next epoch has no outstanding target");
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
fn tick_horizon_is_checked_without_state_mutation() {
    let mut demand = FrameDemand::new(2, plan());
    demand.request();
    let sweep = demand.begin_sweep(Tick(0), ()).expect("first");
    let (written, ()) = write_all(sweep);
    demand
        .finish_written(written, Tick(u64::MAX - 1))
        .expect("active");
    demand.request();

    let before = observable_demand_state(&demand);
    for _ in 0..2 {
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = demand.begin_sweep(Tick::MAX, ());
            }))
            .is_err(),
            "an unrepresentable positive throttle interval panics"
        );
        assert_eq!(
            observable_demand_state(&demand),
            before,
            "the checked Tick horizon rejects before state mutation"
        );
    }
}

#[test]
fn abandoned_epochs_witnesses_are_terminally_rejected() {
    let mut demand = FrameDemand::new(10, plan());
    let old_sweep = begin_after_established_write(&mut demand);
    // Produce the old terminal witness before abandonment. The explicit
    // ui-pass escape, not this positive oracle, owns post-abandon stale start.
    let (old_written, ()) = write_all(old_sweep);
    demand.abandon_active();

    let new_sweep = demand.begin_sweep(ANCHOR_ELIGIBLE, ()).expect("epoch 2");
    assert_eq!(new_sweep.epoch().get(), 2);

    // The machine's guarantee is that the abandoned epoch's already-minted
    // witness is terminally rejected without immediate or hidden mutation.
    let before = observable_demand_state(&demand);
    assert_eq!(
        demand.finish_written(old_written, REJECTION_TIME),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&demand),
        before,
        "stale rejection did not disturb any observable live-sweep state"
    );
    assert_eq!(demand.sweeping(), Some(new_sweep.epoch()));
    assert_rejection_preserved_future(&mut demand, new_sweep, 3);
}
