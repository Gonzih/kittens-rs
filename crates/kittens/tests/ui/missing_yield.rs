use kittens::reactor::Control;
use kittens::source::FixedQueue;

struct Sources {
    model: FixedQueue<u8, 4>,
    input: FixedQueue<u8, 4>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(model)]
        #[readiness(may_remain_ready)]
        _ = sources.model => { Ok(Control::Continue) }

        #[source(input)]
        #[readiness(may_remain_ready)]
        _ = sources.input => { Ok(Control::Continue) }
    }
}

fn main() {}
