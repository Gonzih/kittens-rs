//! K2R-0 state-table and coverage oracles for the witness-driven
//! demand/sweep machine (exit-review round 1 API: crate-owned `Sweep<S>`,
//! provenance-branded settlement, invalidation terminating the affected
//! epoch, abandon recovery, written-milestone vocabulary).
//!
//! Stripes are settled here by running a real model transfer per stripe.
//! Target-driven start and the mandatory written-or-unwritten settlement
//! exercise the full transfer→sweep→demand composition.

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use kittens_render::demand::{ForeignSweep, FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{FrameEpoch, PanelGeometry, Region};
use kittens_render::sweep::{InvalidPlan, StripeSettlement, StripeTarget, Sweep, SweepPlan};
use kittens_render::transfer::{OwnedTransfer, Recovered, TransferOutcome};

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

fn observable_demand_state(demand: &FrameDemand) -> (bool, Option<FrameEpoch>, bool, Option<Tick>) {
    (
        demand.is_dirty(),
        demand.sweeping(),
        demand.full_repaint_required(),
        demand.eligible_at(),
    )
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

/// Drives one already-issued target to the requested model outcome and
/// returns its mandatory move-only settlement witness.
fn transfer_target(target: StripeTarget, outcome: TransferOutcome) -> StripeSettlement {
    let expected_region = target.region();
    let hw = Arc::new(Hw {
        done: Mutex::new(false),
        fail: Mutex::new(false),
    });
    let mut flight = target
        .start_flight((), |region| {
            assert_eq!(region, expected_region, "target supplies the start region");
            Ok::<_, core::convert::Infallible>(ModelTransfer {
                hw: Arc::clone(&hw),
                resources: Some(((), ())),
                settled: None,
            })
        })
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
    let (aborted, ()) = sweep.abort();
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
    let (aborted, ()) = sweep.abort();
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
    let before = observable_demand_state(&right);
    assert_eq!(
        right.finish_written(left_written, Tick(1)),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&right),
        before,
        "every observable stays unchanged on foreign written rejection"
    );

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
fn foreign_failed_settlement_is_rejected_without_mutation() {
    let mut left = demand();
    let mut right = demand();
    left.request();
    right.request();

    let left_sweep = left.begin_sweep(Tick(0), ()).expect("left sweep");
    let right_sweep = right.begin_sweep(Tick(0), ()).expect("right sweep");
    let (left_aborted, ()) = left_sweep.abort();
    let (right_aborted, ()) = right_sweep.abort();

    let before = observable_demand_state(&right);
    assert_eq!(
        right.finish_failed(left_aborted, Tick(1)),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&right),
        before,
        "foreign abort cannot disturb the active demand state"
    );
    right
        .finish_failed(right_aborted, Tick(1))
        .expect("rightful abort still settles");
    left.abandon_active();
}

#[test]
fn stale_failed_settlement_is_rejected_without_mutation() {
    let mut demand = demand();
    demand.request();
    let old = demand.begin_sweep(Tick(0), ()).expect("old");
    let (old_aborted, ()) = old.abort();
    demand.abandon_active();
    let replacement = demand.begin_sweep(Tick(0), ()).expect("replacement");
    let (replacement_aborted, ()) = replacement.abort();

    let before = observable_demand_state(&demand);
    assert_eq!(
        demand.finish_failed(old_aborted, Tick(1)),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&demand),
        before,
        "stale abort rejection leaves all observable state unchanged"
    );
    demand
        .finish_failed(replacement_aborted, Tick(1))
        .expect("replacement abort remains valid");
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
    let _second = sweep
        .next_target()
        .expect("settlement cleared outstanding for the next position");

    let (aborted, ()) = sweep.abort();
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

    let (left_aborted, ()) = left.abort();
    left_demand
        .finish_failed(left_aborted, Tick(1))
        .expect("left abort");
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
    let before = observable_demand_state(&demand);
    assert_eq!(
        demand.finish_written(old_written, Tick(1)),
        Err(ForeignSweep)
    );
    assert_eq!(
        observable_demand_state(&demand),
        before,
        "stale rejection did not disturb any observable live-sweep state"
    );
    assert_eq!(demand.sweeping(), Some(new_sweep.epoch()));

    let (written, ()) = write_all(new_sweep);
    demand.finish_written(written, Tick(1)).expect("active");
}
