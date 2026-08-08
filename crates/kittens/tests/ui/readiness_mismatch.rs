use kittens::reactor::Control;
use kittens::source::FixedQueue;

struct Sources {
    stream: FixedQueue<u8, 4>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stream)]
        #[readiness(quiescent)]
        _ = sources.stream => { Ok(Control::Continue) }
    }
}

fn main() {}
