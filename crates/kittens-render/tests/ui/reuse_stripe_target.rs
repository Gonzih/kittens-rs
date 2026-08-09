use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::{InFlight, OwnedTransfer};

fn reuse<X: OwnedTransfer, S>(
    first: X,
    first_spare: S,
    second: X,
    second_spare: S,
    target: StripeTarget,
) {
    let _first = InFlight::new(first, first_spare, target);
    let _second = InFlight::new(second, second_spare, target);
}

fn main() {}
