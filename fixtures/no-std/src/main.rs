#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

use core::convert::Infallible;
#[cfg(target_os = "none")]
use core::future::Future;
#[cfg(target_os = "none")]
use core::panic::PanicInfo;
#[cfg(target_os = "none")]
use core::task::{Context, Waker};

use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

#[derive(Clone, Copy)]
enum Exit {
    Stop,
}

struct Sources {
    stop: Latched<()>,
    sensor: FixedQueue<u16, 4>,
}

#[allow(dead_code)]
async fn kernel_path(sources: &mut Sources) -> Result<Exit, Infallible> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => {
            Ok(Exit::Stop)
        }

        #[source(sensor)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "sensor telemetry is best effort")]
        #[drain(max = 4)]
        #[last]
        _sample = sources.sensor => {
            Ok(Control::Continue)
        }
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Poll the generated future so the linked-size fixture measures the reactor
    // path rather than a dead-code-eliminated placeholder.
    let mut sources = Sources {
        stop: Latched::new(),
        sensor: FixedQueue::new(),
    };
    if core::hint::black_box(false) {
        sources.stop.arm(()).unwrap();
    } else {
        sources.sensor.push(7).unwrap();
    }
    let future = kernel_path(&mut sources);
    let mut future = core::pin::pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    let _ = future.as_mut().poll(&mut cx);

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}
