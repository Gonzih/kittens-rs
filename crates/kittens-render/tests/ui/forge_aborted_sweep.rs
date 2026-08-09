use kittens_render::geometry::FrameEpoch;
use kittens_render::sweep::AbortedSweep;

fn forge(epoch: FrameEpoch) -> AbortedSweep {
    AbortedSweep {
        demand_id: 0,
        epoch,
    }
}

fn main() {}
