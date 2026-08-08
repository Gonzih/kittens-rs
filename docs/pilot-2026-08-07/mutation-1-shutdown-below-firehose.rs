//! Event loop for a small status daemon: a stop latch and a buffered event
//! queue feeding a counter.

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

        #[source(events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "no lower interactive source in this loop")]
        event = sources.events => {
            if event == 42 {
                Ok(Control::Stop(event))
            } else {
                Ok(Control::Continue)
            }
        }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => {
            Ok(0)
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
