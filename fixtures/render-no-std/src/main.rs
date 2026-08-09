#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
use core::task::{Context, Poll, Waker};

use kittens_render::demand::{FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::PanelGeometry;
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

#[derive(Debug)]
struct ProbeTransport {
    sequence: u8,
}

#[derive(Debug)]
struct ProbeTransfer {
    transport: ProbeTransport,
    buffer: [u8; 1],
    outcome: TransferOutcome,
}

impl OwnedTransfer for ProbeTransfer {
    type Transport = ProbeTransport;
    type Buffer = [u8; 1];

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }

    fn cancel(&mut self) {
        self.outcome = TransferOutcome::Cancelled;
    }

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.transport,
            buffer: self.buffer,
            outcome: self.outcome,
        }
    }
}

/// Exercises the public render proof chain from a separate crate so the
/// bare-metal gate links real downstream use rather than only building the
/// profile rlib.
fn linked_render_path(stripe_height: u16) -> bool {
    let Ok(plan) = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, stripe_height) else {
        return false;
    };

    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let Some(mut sweep) = demand.begin_sweep(Tick(0), 0xa5_u8) else {
        return false;
    };
    let Some(target) = sweep.next_target() else {
        return false;
    };

    let transfer = ProbeTransfer {
        transport: ProbeTransport { sequence: 1 },
        buffer: [0x5a],
        outcome: TransferOutcome::Completed,
    };
    let mut in_flight = InFlight::new(transfer, [0xc3], target);
    let mut cx = Context::from_waker(Waker::noop());
    let Poll::Ready(mut settled) = in_flight.poll_complete(&mut cx) else {
        return false;
    };
    let Some(written_stripe) = settled.stripe_written() else {
        return false;
    };
    if sweep.mark_written(written_stripe).is_err() {
        return false;
    }

    let (transport, sent, spare) = settled.into_resources();
    let Ok((written_sweep, snapshot)) = sweep.finish() else {
        return false;
    };
    let Ok(disposition) = demand.finish_written(written_sweep, Tick(1)) else {
        return false;
    };

    disposition == WrittenDisposition::Effective
        && !demand.is_dirty()
        && demand.sweeping().is_none()
        && transport.sequence == 1
        && sent == [0x5a]
        && spare == [0xc3]
        && snapshot == 0xa5
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
    // Keep the linked downstream proof-chain path live in the optimized
    // binary; this fixture is evidence about a consumer link, not only type
    // checking.
    let linked = linked_render_path(core::hint::black_box(448));
    core::hint::black_box(linked);

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    assert!(linked_render_path(448));
}
