#![no_std]

use core::convert::Infallible;

use cats::source::Latched;

struct Sources {
    stop: Latched<()>,
}

#[allow(dead_code)]
async fn renamed_dependency(sources: &mut Sources) -> Result<(), Infallible> {
    cats::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => { Ok(()) }
    }
}
