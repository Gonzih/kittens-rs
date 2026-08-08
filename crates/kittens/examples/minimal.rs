//! Minimal executable reactor with one shutdown latch and one buffered source.

#![allow(clippy::ignored_unit_patterns)]

use std::convert::Infallible;

use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

struct Sources {
    stop: Latched<()>,
    events: FixedQueue<u8, 8>,
}

async fn run(sources: &mut Sources) -> Result<u8, Infallible> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => {
            Ok(0)
        }

        #[source(events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "the example has no lower interactive source")]
        #[drain(max = 4)]
        #[last]
        event = sources.events => {
            if event == 42 {
                Ok(Control::Stop(event))
            } else {
                Ok(Control::Continue)
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut sources = Sources {
        stop: Latched::new(),
        events: FixedQueue::new(),
    };
    sources.events.push(1).unwrap();
    sources.events.push(42).unwrap();
    assert_eq!(run(&mut sources).await, Ok(42));
}
