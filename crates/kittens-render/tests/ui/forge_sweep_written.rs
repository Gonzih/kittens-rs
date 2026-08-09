use kittens_render::geometry::FrameEpoch;
use kittens_render::sweep::SweepWritten;

fn forge(epoch: FrameEpoch) -> SweepWritten {
    SweepWritten {
        demand_id: 0,
        epoch,
    }
}

fn main() {}
