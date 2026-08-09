use kittens_render::geometry::{FrameEpoch, Region};
use kittens_render::sweep::StripeTarget;

fn forge(epoch: FrameEpoch) -> StripeTarget {
    StripeTarget {
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
