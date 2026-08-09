use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::{InFlight, OwnedTransfer};

fn pair_independently<X: OwnedTransfer, S>(transfer: X, spare: S, target: StripeTarget) {
    let _flight = InFlight::new(transfer, spare, target);
}

fn main() {}
