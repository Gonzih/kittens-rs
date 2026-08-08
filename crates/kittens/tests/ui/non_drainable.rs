use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    touch: Latched<u8>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(touch)]
        #[readiness(quiescent)]
        #[drain(max = 4)]
        _ = sources.touch => { Ok(Control::Continue) }
    }
}

fn main() {}
