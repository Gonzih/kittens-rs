//! Negative control: `Sweep` owns its snapshot and exposes only `&S`, but
//! ordinary interior mutability remains a documented compiling escape.

use core::cell::Cell;

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::PanelGeometry;
use kittens_render::sweep::SweepPlan;

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let sweep = demand
        .begin_sweep(Tick(0), Cell::new(1_u8))
        .expect("requested sweep");
    sweep.snapshot().set(2);
    assert_eq!(sweep.snapshot().get(), 2);
}
