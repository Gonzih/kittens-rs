//! Event loop for a job monitor: a stop latch and a one-time readiness probe
//! whose completion reports the job result.

#![allow(clippy::ignored_unit_patterns)]

use std::convert::Infallible;

use kittens::source::Latched;

async fn probe_job() -> u8 {
    tokio::task::yield_now().await;
    42
}

struct Sources {
    stop: Latched<()>,
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

        #[source(job)]
        #[readiness(quiescent)]
        #[terminal]
        result = probe_job() => {
            Ok(result)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut sources = Sources {
        stop: Latched::new(),
    };
    assert_eq!(run(&mut sources).await, Ok(42));
}
