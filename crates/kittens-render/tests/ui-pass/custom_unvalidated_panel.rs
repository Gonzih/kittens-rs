//! Negative control: boards outside the admitted set deliberately retain a
//! loudly named geometry escape; the type system cannot validate hardware.

use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::SweepPlan;

fn main() {
    let geometry = PanelGeometry::custom_unvalidated_panel(Region {
        x: 12,
        y: 34,
        width: 8,
        height: 4,
    });
    let plan = SweepPlan::for_panel(geometry, 2).expect("valid custom plan");
    assert_eq!(plan.stripe_count(), 2);
}
