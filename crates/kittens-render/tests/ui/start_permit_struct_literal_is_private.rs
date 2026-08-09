use kittens_render::transfer::StartPermit;

// The permit's field privacy is the whole guarantee: pin it independently
// so a future refactor cannot quietly make the permit constructible.
fn forge(key: &'static mut ()) -> StartPermit<'static> {
    StartPermit { _key: key }
}

fn main() {}
