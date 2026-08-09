use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::{InFlight, OwnedTransfer};

fn construct_directly<X: OwnedTransfer, S>(transfer: X, spare: S, target: StripeTarget) {
    let _flight = InFlight {
        transfer: Some(transfer),
        spare: Some(spare),
        draining: false,
        target: Some(target),
    };
}

fn main() {}
