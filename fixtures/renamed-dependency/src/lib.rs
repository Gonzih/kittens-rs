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

        /// The renamed facade must still route an armed shutdown source through
        /// the terminal handler without requiring the canonical crate spelling.
        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => { Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::future::Future;
    use core::task::{Context, Poll, Waker};
    use std::boxed::Box;

    use super::{Sources, renamed_dependency};
    use cats::source::Latched;

    #[test]
    fn renamed_facade_executes_an_armed_shutdown_source() {
        let mut stop = Latched::new();
        stop.arm(()).expect("a dormant latch accepts one event");
        let mut sources = Sources { stop };
        let mut reactor = Box::pin(renamed_dependency(&mut sources));
        let mut cx = Context::from_waker(Waker::noop());

        assert_eq!(reactor.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
    }
}
