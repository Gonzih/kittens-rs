//! Event loop for a sensor hub: a stop latch and a latched interrupt-style
//! sample source.

#![allow(clippy::ignored_unit_patterns)]

use std::convert::Infallible;

use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    stop: Latched<()>,
    sample: Latched<u8>,
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

        #[source(sample)]
        #[readiness(quiescent)]
        #[drain(max = 4)]
        value = sources.sample => {
            if value == 42 {
                Ok(Control::Stop(value))
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
        sample: Latched::new(),
    };
    sources.sample.arm(42).unwrap();
    assert_eq!(run(&mut sources).await, Ok(42));
}
