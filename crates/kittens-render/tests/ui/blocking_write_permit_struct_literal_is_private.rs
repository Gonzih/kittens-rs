use kittens_render::blocking::BlockingWritePermit;

// The permit's field privacy is independent of its private constructor: pin
// both so a refactor cannot quietly open a second construction spelling.
fn forge(key: &'static mut ()) -> BlockingWritePermit<'static> {
    BlockingWritePermit { _key: key }
}

fn main() {}
