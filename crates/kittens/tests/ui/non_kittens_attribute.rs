struct Sources {
    stop: kittens::source::Latched<()>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        // Doc attributes are the sole source-side exception. In particular,
        // cfg-driven topology must remain visible to the macro and rejected.
        #[cfg(any())]
        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => { Ok(()) }
    }
}

fn main() {}
