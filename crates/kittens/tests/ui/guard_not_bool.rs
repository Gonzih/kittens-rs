use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    event: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        #[when(7)]
        _ = sources.event => { Ok(Control::Continue) }
    }
}

fn main() {}
