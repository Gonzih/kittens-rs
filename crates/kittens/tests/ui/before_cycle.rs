use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    a: Latched<()>,
    b: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(a)]
        #[readiness(quiescent)]
        #[before(b)]
        _ = sources.a => { Ok(Control::Continue) }

        #[source(b)]
        #[readiness(quiescent)]
        #[before(a)]
        _ = sources.b => { Ok(Control::Continue) }
    }
}

fn main() {}
