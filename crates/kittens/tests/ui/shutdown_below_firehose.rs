use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

struct Sources {
    firehose: FixedQueue<u8, 4>,
    cancel: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(firehose)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "firehose ordering mutation fixture")]
        _ = sources.firehose => { Ok(Control::Continue) }

        #[source(cancel)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.cancel => { Ok(()) }
    }
}

fn main() {}
