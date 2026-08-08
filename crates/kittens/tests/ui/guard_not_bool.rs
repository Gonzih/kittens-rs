use kittens::reactor::Control;
use kittens::source::Latched;

struct Sources {
    event: Latched<()>,
    // Unguarded so the all-guarded KTR014 rejection does not mask the
    // guard-type oracle this fixture exists to exercise.
    idle: Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        #[when(7)]
        _ = sources.event => { Ok(Control::Continue) }

        #[source(idle)]
        #[readiness(quiescent)]
        _ = sources.idle => { Ok(Control::Stop(())) }
    }
}

fn main() {}
