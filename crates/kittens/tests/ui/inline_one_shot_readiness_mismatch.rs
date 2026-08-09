use core::future::Ready;

use kittens::reactor::Control;
use kittens::source::OptionalInlineOneShot;

struct Sources {
    completion: OptionalInlineOneShot<Ready<u8>>,
}

async fn run(sources: &mut Sources) -> Result<(), ()> {
    kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(completion)]
        #[readiness(may_remain_ready)]
        _ = sources.completion => { Ok(Control::Continue) }
    }
}

fn main() {}
