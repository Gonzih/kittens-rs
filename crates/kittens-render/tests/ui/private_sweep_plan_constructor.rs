use kittens_render::geometry::Region;
use kittens_render::sweep::SweepPlan;

fn main() {
    let _plan = SweepPlan::new(
        Region {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        1,
    );
}
