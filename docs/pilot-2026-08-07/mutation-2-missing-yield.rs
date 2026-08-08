//! Event loop for a chat client: a stop latch, a model-token stream, and a
//! user-input queue that must stay responsive while tokens stream.

#![allow(clippy::ignored_unit_patterns)]

use std::convert::Infallible;

use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

struct Sources {
    stop: Latched<()>,
    model: FixedQueue<u8, 8>,
    input: FixedQueue<u8, 4>,
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

        #[source(model)]
        #[readiness(may_remain_ready)]
        token = sources.model => {
            if token == 42 {
                Ok(Control::Stop(token))
            } else {
                Ok(Control::Continue)
            }
        }

        #[source(input)]
        #[readiness(may_remain_ready)]
        key = sources.input => {
            if key == 9 {
                Ok(Control::Stop(key))
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
        model: FixedQueue::new(),
        input: FixedQueue::new(),
    };
    sources.model.push(42).unwrap();
    sources.input.push(9).unwrap();
    let exit = run(&mut sources).await.unwrap();
    assert!(exit == 42 || exit == 9);
}
