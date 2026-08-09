use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use kittens::reactor::Control;
use kittens::source::OptionalInlineOneShot;

// Admission of the carrier cannot prove that an arbitrary inner future
// registers a wake or ever completes. This deliberately broken future does
// neither, and remains an explicit compiling honesty boundary.
struct InertFuture;

impl Future for InertFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

struct Sources {
    completion: OptionalInlineOneShot<InertFuture>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: [before_poll]; }

        before_poll {
            // `future_mut` deliberately exposes ordinary `&mut F`: even
            // replacement of the retained operation remains unchecked Rust.
            if let Some(future) = sources.completion.future_mut() {
                let _replaced = core::mem::replace(future, InertFuture);
            }
            Ok(())
        }

        /// This source is admitted because the carrier retains its future;
        /// the kernel cannot certify the inert future's producer behavior.
        #[source(completion)]
        #[readiness(quiescent)]
        _ = sources.completion => { Ok(Control::Continue) }
    }
}

fn main() {
    let _sources = Sources {
        completion: OptionalInlineOneShot::from_future(InertFuture),
    };
}
