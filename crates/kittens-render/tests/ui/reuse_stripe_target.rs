use kittens_render::geometry::Region;
use kittens_render::sweep::StripeTarget;
use kittens_render::transfer::{FlightStarter, OwnedTransfer, StartPermit};

struct Accept<X>(X);

impl<X: OwnedTransfer> FlightStarter for Accept<X> {
    type Transfer = X;
    type Error = ();

    fn start(
        self,
        _region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        Ok(self.0)
    }
}

fn reuse<X: OwnedTransfer, S>(
    first: X,
    first_spare: S,
    second: X,
    second_spare: S,
    target: StripeTarget,
) {
    let _first = target.start_flight(first_spare, Accept(first));
    let _second = target.start_flight(second_spare, Accept(second));
}

fn main() {}
