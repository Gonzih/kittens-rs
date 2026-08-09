use kittens_render::geometry::{FrameEpoch, Region};
use kittens_render::sweep::StripeUnwritten;
use kittens_render::transfer::TransferOutcome;

fn forge(epoch: FrameEpoch) -> StripeUnwritten {
    StripeUnwritten {
        demand_id: 0,
        epoch,
        region: Region {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        outcome: TransferOutcome::Failed,
    }
}

fn main() {}
