#![allow(dead_code, unused_imports, unused_variables)]

use std::collections::VecDeque;

use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

struct Pair {
    stream: FixedQueue<u8, 4>,
    cancel_looking: Latched<()>,
}

// Removing `shutdown` erases the semantic fact, so this cancellation-looking
// arm may legally appear below the stream.
async fn removed_shutdown(sources: &mut Pair) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "constraint erasure intentionally compiles")]
        _ = sources.stream => { Ok(Control::Continue) }

        #[source(cancel_looking)]
        #[readiness(quiescent)]
        #[starvation(allowed, reason = "constraint erasure intentionally compiles")]
        _ = sources.cancel_looking => { Ok(Control::Stop(())) }
    }
}

struct VoicePair {
    voice: Latched<()>,
    telemetry: Latched<()>,
}

// Removing `last` means the macro cannot infer that voice belongs last.
async fn removed_last(sources: &mut VoicePair) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(voice)]
        #[readiness(quiescent)]
        _ = sources.voice => { Ok(Control::Continue) }

        #[source(telemetry)]
        #[readiness(quiescent)]
        _ = sources.telemetry => { Ok(Control::Stop(())) }
    }
}

struct Waived {
    model: FixedQueue<u8, 4>,
    input: FixedQueue<u8, 4>,
}

// The explicit waiver weakens the input guarantee and therefore compiles.
async fn waived_input(sources: &mut Waived) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(model)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "negative control intentionally weakens input")]
        _ = sources.model => { Ok(Control::Continue) }

        #[source(input)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "negative control intentionally weakens input")]
        _ = sources.input => { Ok(Control::Stop(())) }
    }
}

struct One {
    event: Latched<()>,
}

// Removing both the phase block and requirement leaves no intent to infer.
async fn removed_phase_and_requirement(sources: &mut One) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => { Ok(Control::Stop(())) }
    }
}

// Handler interiors remain ordinary Rust: manual unbounded draining is outside
// the macro-managed bound.
async fn manual_handler_loop(sources: &mut One, raw: &mut VecDeque<u8>) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => {
            while raw.pop_front().is_some() {}
            Ok(Control::Stop(()))
        }
    }
}

// Raw handler-side replacement is an explicit ambient-Rust boundary.
async fn raw_replacement(sources: &mut One) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => {
            sources.event = Latched::new();
            Ok(Control::Stop(()))
        }
    }
}

// Awaiting an operation inside a handler is not source admission.
async fn unchecked_handler_await(sources: &mut One) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => {
            core::future::pending::<()>().await;
            Ok(Control::Stop(()))
        }
    }
}

// Phase placement does not make the phase bounded or preemptible.
async fn unchecked_phase_await(sources: &mut One) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: [before_poll]; }

        before_poll {
            core::future::pending::<()>().await;
            Ok(())
        }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => { Ok(Control::Stop(())) }
    }
}

struct RawWriter;

impl RawWriter {
    fn write(&mut self) {}
}

// The kernel does not own rendering authority or infer single-flight state.
async fn duplicate_raw_writes(sources: &mut One, writer: &mut RawWriter) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(event)]
        #[readiness(quiescent)]
        _ = sources.event => {
            writer.write();
            writer.write();
            Ok(Control::Stop(()))
        }
    }
}

// Runtime mode truth and descriptive "double buffer" comments do not change
// the legal program. This deliberately leaves a frame source armed in Off mode.
fn descriptive_state_does_not_constrain_sources(source: &mut Latched<()>, off: bool) {
    // This application claims to use a double buffer.
    if off {
        source.arm(()).unwrap();
    }
}

fn main() {}
