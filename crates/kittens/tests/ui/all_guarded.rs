use kittens::reactor::Control;
use kittens::source::FixedQueue;

struct Sources {
    commands: FixedQueue<u8, 4>,
    telemetry: FixedQueue<u8, 4>,
}

struct App {
    accepts_commands: bool,
    accepts_telemetry: bool,
}

// Every arm carries `#[when]`. If both guards snapshot false in one
// arbitration, no source is polled, no waker is registered, and the reactor
// pends forever. The macro rejects the topology instead.
async fn run(app: &mut App, sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(commands)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "fixture exercises the all-guarded rejection")]
        #[when(app.accepts_commands)]
        _ = sources.commands => { Ok(Control::Continue) }

        #[source(telemetry)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "fixture exercises the all-guarded rejection")]
        #[when(app.accepts_telemetry)]
        _ = sources.telemetry => { Ok(Control::Stop(())) }
    }
}

fn main() {}
