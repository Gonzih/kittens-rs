//! Negative control: `abandon_active` terminally rejects the old epoch in
//! the demand machine, but ordinary Rust cannot invalidate a retained sweep.
//! It can still mint an old target after the replacement begins; callers must
//! drop unstarted targets, drain started work, force repaint, and invalidate a
//! replacement that a late write may overlap.

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::PanelGeometry;
use kittens_render::sweep::SweepPlan;

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut old_sweep = demand.begin_sweep(Tick(0), 0_u8).expect("old sweep");

    demand.abandon_active();
    let replacement = demand
        .begin_sweep(Tick(0), 1_u8)
        .expect("replacement sweep");
    let stale_target = old_sweep
        .next_target()
        .expect("retained old sweep can still mint after replacement");

    let _all_values_remain_live = (old_sweep, stale_target, replacement);
}
