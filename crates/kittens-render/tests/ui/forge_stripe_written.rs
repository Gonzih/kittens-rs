use kittens_render::geometry::{FrameEpoch, Region};
use kittens_render::sweep::StripeWritten;

fn forge(epoch: FrameEpoch) -> StripeWritten {
    StripeWritten {
        demand_id: 0,
        epoch,
        region: Region {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
    }
}

fn main() {}
