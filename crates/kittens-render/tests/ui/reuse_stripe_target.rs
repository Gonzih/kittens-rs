use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::OwnedTransfer;

fn reuse<X: OwnedTransfer, S>(
    first: X,
    first_spare: S,
    second: X,
    second_spare: S,
    target: StripeTarget,
) {
    let _first = target.start_flight(first_spare, |_| Ok::<X, ()>(first));
    let _second = target.start_flight(second_spare, |_| Ok::<X, ()>(second));
}

fn main() {}
