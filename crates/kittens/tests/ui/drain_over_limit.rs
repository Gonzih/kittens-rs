use kittens::reactor::Control;
use kittens::source::FixedQueue;

struct Sources {
    stream: FixedQueue<u8, 4>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "single source drain mutation fixture")]
        #[drain(max = 4097)]
        _ = sources.stream => { Ok(Control::Continue) }
    }
}

fn main() {}
