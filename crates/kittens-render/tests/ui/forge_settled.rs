use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::{Settled, TransferOutcome};

fn forge(target: StripeTarget) -> Settled<(), (), ()> {
    Settled {
        transport: (),
        buffer: (),
        spare: (),
        outcome: TransferOutcome::Completed,
        target,
    }
}

fn main() {}
