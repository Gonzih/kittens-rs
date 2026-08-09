//! K2R-0 state-table and coverage oracles for `FrameDemand` and the
//! full-panel sweep plan. Each normative row of the demand table has a
//! trace; coverage-by-construction has both positive and rejection traces.

#![allow(missing_docs)]

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::Region;
use kittens_render::sweep::{InvalidPlan, SweepPlan, SweepProgress};

const PANEL: Region = Region {
    x: 0,
    y: 0,
    width: 368,
    height: 448,
};

fn plan() -> SweepPlan {
    SweepPlan::new(PANEL, 16).expect("valid plan")
}

/// Drives a sweep to full coverage and returns the completed witness.
fn cover(mut progress: SweepProgress) -> kittens_render::sweep::CompletedSweep {
    while let Some(region) = progress.next_region() {
        progress.mark_written(region).expect("in-order mark");
    }
    progress.complete().expect("fully covered")
}

#[test]
fn clean_demand_mints_nothing() {
    let mut demand = FrameDemand::new(0);
    assert!(demand.begin_sweep(Tick(0)).is_none(), "clean → no sweep");
    assert!(!demand.is_dirty());
}

#[test]
fn requests_coalesce_into_one_sweep_with_monotonic_epochs() {
    let mut demand = FrameDemand::new(0);
    demand.request();
    demand.request();
    demand.request();

    let first = demand.begin_sweep(Tick(0)).expect("dirty → sweep");
    assert!(!demand.is_dirty(), "demand consumed into the sweep");
    assert!(demand.begin_sweep(Tick(0)).is_none(), "one sweep in flight");

    let completed = cover(SweepProgress::new(plan(), first));
    demand.finish_presented(completed, Tick(1));

    demand.request();
    let second = demand.begin_sweep(Tick(1)).expect("second sweep");
    assert_eq!(second.epoch().get(), 1, "epochs are strictly monotonic");
    let _ = second;
}

#[test]
fn request_during_sweep_targets_the_next_epoch() {
    let mut demand = FrameDemand::new(0);
    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("sweep");

    demand.request(); // arrives mid-sweep; must survive settlement
    let completed = cover(SweepProgress::new(plan(), token));
    demand.finish_presented(completed, Tick(1));

    assert!(demand.is_dirty(), "mid-sweep demand survives");
    let next = demand.begin_sweep(Tick(1)).expect("next sweep");
    assert_eq!(next.epoch().get(), 1);
    let _ = next;
}

#[test]
fn throttle_blocks_schedules_and_begin_sweep_acknowledges() {
    let mut demand = FrameDemand::new(10);
    demand.request();
    let token = demand
        .begin_sweep(Tick(0))
        .expect("first sweep unthrottled");
    demand.finish_presented(cover(SweepProgress::new(plan(), token)), Tick(0));

    demand.request();
    assert!(
        demand.begin_sweep(Tick(5)).is_none(),
        "inside the throttle window"
    );
    assert_eq!(
        demand.eligible_at(),
        Some(Tick(10)),
        "eligibility scheduled at last_present + interval"
    );

    let token = demand
        .begin_sweep(Tick(10))
        .expect("eligible exactly at the scheduled instant");
    assert_eq!(
        demand.eligible_at(),
        None,
        "begin_sweep is the sole acknowledgment; schedule cleared"
    );
    drop(token);
}

#[test]
fn eligible_at_is_masked_while_sweeping_or_clean() {
    let mut demand = FrameDemand::new(10);
    assert_eq!(demand.eligible_at(), None, "clean → no eligibility");

    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("sweep");
    demand.request();
    assert_eq!(
        demand.eligible_at(),
        None,
        "an active sweep masks eligibility"
    );
    demand.finish_failed(SweepProgress::new(plan(), token).abort(), Tick(1));
}

#[test]
fn failed_sweep_retains_demand_sets_full_repaint_and_keeps_throttle() {
    let mut demand = FrameDemand::new(10);
    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("first sweep");
    demand.finish_presented(cover(SweepProgress::new(plan(), token)), Tick(0));
    assert!(
        !demand.full_repaint_required(),
        "clean presented sweep clears the initial obligation"
    );

    demand.request();
    let token = demand.begin_sweep(Tick(10)).expect("second sweep");
    let aborted = SweepProgress::new(plan(), token).abort();
    demand.finish_failed(aborted, Tick(11));

    assert!(demand.is_dirty(), "failed sweep retains demand");
    assert!(
        demand.full_repaint_required(),
        "failure forces full repaint"
    );
    // Throttle did not advance to Tick(11): the next sweep is eligible at
    // last_present(0) + 10 = 10, so it begins immediately at Tick(11).
    assert!(
        demand.begin_sweep(Tick(11)).is_some(),
        "failure did not advance the throttle"
    );
}

#[test]
fn invalidation_during_sweep_survives_that_sweeps_presentation() {
    let mut demand = FrameDemand::new(0);
    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("sweep");

    // Transport reset mid-sweep: this sweep's output is suspect.
    demand.invalidate();

    let completed = cover(SweepProgress::new(plan(), token));
    demand.finish_presented(completed, Tick(1));
    assert!(
        demand.full_repaint_required(),
        "a sweep minted before the invalidation cannot clear the obligation"
    );
    assert!(demand.is_dirty(), "invalidation raised demand");

    // The next, post-invalidation sweep does clear it.
    let token = demand.begin_sweep(Tick(1)).expect("repaint sweep");
    demand.finish_presented(cover(SweepProgress::new(plan(), token)), Tick(2));
    assert!(!demand.full_repaint_required());
}

#[test]
fn plan_tiles_the_panel_exactly_including_partial_last_stripe() {
    // 448 / 16 = 28 exact stripes; also check a non-dividing height.
    let exact = SweepPlan::new(PANEL, 16).expect("valid");
    assert_eq!(exact.stripe_count(), 28);

    let uneven = SweepPlan::new(PANEL, 30).expect("valid");
    assert_eq!(uneven.stripe_count(), 15, "ceil(448 / 30)");
    let mut covered = 0u32;
    let mut expected_y = PANEL.y;
    for index in 0..uneven.stripe_count() {
        let region = uneven.region_at(index).expect("in range");
        assert_eq!(region.x, PANEL.x);
        assert_eq!(region.width, PANEL.width, "full panel width");
        assert_eq!(region.y, expected_y, "stripes are contiguous");
        expected_y += region.height;
        covered += u32::from(region.height);
    }
    assert_eq!(
        covered,
        u32::from(PANEL.height),
        "exact coverage, no gap/overlap"
    );
    assert!(uneven.region_at(15).is_none());
}

#[test]
fn out_of_order_and_repeated_marks_are_rejected() {
    let mut demand = FrameDemand::new(0);
    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("sweep");
    let mut progress = SweepProgress::new(plan(), token);

    let first = progress.next_region().expect("first stripe");
    let second_region = plan().region_at(1).expect("second stripe");

    // Skipping ahead is rejected.
    let error = progress
        .mark_written(second_region)
        .expect_err("out-of-order mark");
    assert_eq!(error.expected, Some(first));

    // Marking the right one succeeds; repeating it is rejected.
    progress.mark_written(first).expect("in order");
    let error = progress.mark_written(first).expect_err("repeat mark");
    assert_eq!(error.expected, Some(second_region));
}

#[test]
fn completion_requires_full_coverage() {
    let mut demand = FrameDemand::new(0);
    demand.request();
    let token = demand.begin_sweep(Tick(0)).expect("sweep");
    let mut progress = SweepProgress::new(plan(), token);

    let first = progress.next_region().expect("first stripe");
    progress.mark_written(first).expect("in order");

    // One stripe is not a frame: completion refuses.
    let progress = progress.complete().expect_err("incomplete coverage");
    assert!(!progress.is_complete());

    // Abort is always available and settles the demand machine as failed.
    demand.finish_failed(progress.abort(), Tick(1));
    assert!(demand.is_dirty());
}

#[test]
fn invalid_plans_are_rejected() {
    assert_eq!(
        SweepPlan::new(
            Region {
                x: 0,
                y: 0,
                width: 0,
                height: 448
            },
            16
        )
        .unwrap_err(),
        InvalidPlan::EmptyPanel
    );
    assert_eq!(
        SweepPlan::new(PANEL, 0).unwrap_err(),
        InvalidPlan::ZeroStripe
    );
}
