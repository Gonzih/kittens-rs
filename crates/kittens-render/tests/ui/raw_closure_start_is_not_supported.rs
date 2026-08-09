use kittens_render::geometry::Region;
use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::OwnedTransfer;

fn start_with_raw_closure<X: OwnedTransfer, S>(transfer: X, spare: S, target: StripeTarget) {
    let _flight =
        target.start_flight(spare, move |_region: Region| Ok::<X, ()>(transfer));
}

fn main() {}
