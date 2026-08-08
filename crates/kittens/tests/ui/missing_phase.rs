use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    event: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: [after_event]; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => { Ok(Control::Continue) }
    }
}

fn main() {}
