use kittens_render::geometry::Region;
use kittens_render::transfer::{FlightStarter, OwnedTransfer, StartPermit};

struct LeakPermit<X>(X);

impl<X: OwnedTransfer> FlightStarter for LeakPermit<X> {
    type Transfer = X;
    type Error = StartPermit<'static>;

    fn start(
        self,
        _region: Region,
        permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        Err(permit)
    }
}

fn main() {}
