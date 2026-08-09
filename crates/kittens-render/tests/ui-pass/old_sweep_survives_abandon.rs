//! Negative control: `abandon_active` terminally rejects the old epoch in
//! the demand machine, but ordinary Rust cannot invalidate a retained sweep
//! value. Draining old physical transfers remains a caller obligation.

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::PanelGeometry;
use kittens_render::sweep::SweepPlan;

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let old_sweep = demand.begin_sweep(Tick(0), 0_u8).expect("old sweep");

    demand.abandon_active();
    let replacement = demand
        .begin_sweep(Tick(0), 1_u8)
        .expect("replacement sweep");

    let _both_values_remain_live = (old_sweep, replacement);
}
