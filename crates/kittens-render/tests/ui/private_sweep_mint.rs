use kittens_render::geometry::FrameEpoch;
use kittens_render::sweep::{Sweep, SweepPlan};

fn forge(plan: SweepPlan, epoch: FrameEpoch) -> Sweep<()> {
    Sweep::mint((), plan, true, 0, epoch)
}

fn main() {}
