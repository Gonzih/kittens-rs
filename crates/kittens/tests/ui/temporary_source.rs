use kittens::reactor::Control;

async fn run() -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(packet)]
        #[readiness(quiescent)]
        _ = core::future::pending::<u8>() => { Ok(Control::Continue) }
    }
}

fn main() {}
