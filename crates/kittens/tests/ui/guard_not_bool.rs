use kittens::reactor::Control;
use kittens::source::{Cancellation, Latched};

struct Sources {
    event: Latched<()>,
    // A wake-capable unguarded source keeps KTR014 from masking the guard-type
    // oracle this fixture exists to exercise.
    shutdown: Cancellation,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(shutdown)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.shutdown => { Ok(()) }

        #[source(event)]
        #[readiness(quiescent)]
        #[when(7)]
        _ = sources.event => { Ok(Control::Continue) }
    }
}

fn main() {}
