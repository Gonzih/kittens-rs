#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
use core::task::{Context, Poll, Waker};

use kittens_render::demand::{FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{FlightStarter, OwnedTransfer, Recovered, TransferOutcome};

#[derive(Debug)]
struct ProbeTransport {
    sequence: u8,
    written_region: Option<Region>,
}

impl ProbeTransport {
    /// The fixture's concrete start boundary records the exact region it
    /// accepts before returning an owned completion. It is invoked only by
    /// `StripeTarget::start_flight`, so the fixture no longer preclassifies an
    /// unrelated transfer as completed and attaches a target afterward.
    fn start_region(
        mut self,
        region: Region,
        buffer: [u8; 1],
    ) -> Result<ProbeTransfer, (Self, [u8; 1])> {
        self.sequence += 1;
        self.written_region = Some(region);
        Ok(ProbeTransfer {
            transport: self,
            buffer,
            outcome: None,
        })
    }
}

#[derive(Debug)]
struct ProbeTransfer {
    transport: ProbeTransport,
    buffer: [u8; 1],
    outcome: Option<TransferOutcome>,
}

impl OwnedTransfer for ProbeTransfer {
    type Transport = ProbeTransport;
    type Buffer = [u8; 1];

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        if self.outcome.is_none() {
            self.outcome = Some(TransferOutcome::Completed);
        }
        Poll::Ready(())
    }

    fn cancel(&mut self) {
        self.outcome = Some(TransferOutcome::Cancelled);
    }

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.transport,
            buffer: self.buffer,
            outcome: self.outcome.expect("polled or cancelled before recovery"),
        }
    }
}

struct ProbeStart {
    transport: ProbeTransport,
    buffer: [u8; 1],
}

impl FlightStarter for ProbeStart {
    type Transfer = ProbeTransfer;
    type Error = (ProbeTransport, [u8; 1]);

    /// The target supplies the region at the consuming operation boundary;
    /// rejection returns every resource captured by this start attempt.
    fn start(self, region: Region) -> Result<Self::Transfer, Self::Error> {
        self.transport.start_region(region, self.buffer)
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
    let expected_region = target.region();

    let transport = ProbeTransport {
        sequence: 0,
        written_region: None,
    };
    let Ok(mut in_flight) = target.start_flight(
        [0xc3],
        ProbeStart {
            transport,
            buffer: [0x5a],
        },
    ) else {
        return false;
    };
    let mut cx = Context::from_waker(Waker::noop());
    let Poll::Ready(settled) = in_flight.poll_complete(&mut cx) else {
        return false;
    };
    if settled.region() != expected_region {
        return false;
    }

    let (transport, sent, spare, stripe_settlement) = settled.into_parts();
    if sweep.settle(stripe_settlement) != Ok(TransferOutcome::Completed) {
        return false;
    }
    let Ok((written_sweep, snapshot)) = sweep.finish() else {
        return false;
    };
    let Ok(disposition) = demand.finish_written(written_sweep, Tick(1)) else {
        return false;
    };

    if disposition != WrittenDisposition::Effective
        || demand.is_dirty()
        || demand.sweeping().is_some()
        || transport.sequence != 1
        || transport.written_region != Some(expected_region)
        || sent != [0x5a]
        || spare != [0xc3]
        || snapshot != 0xa5
    {
        return false;
    }

    // Shutdown after acceptance must drive the transfer to a real cancelled
    // settlement before the poisoned sweep may abort. Rotate the resources
    // recovered above so the fixture proves both buffers return again.
    demand.request();
    let Some(mut shutdown_sweep) = demand.begin_sweep(Tick(1), 0x3c_u8) else {
        return false;
    };
    let Some(shutdown_target) = shutdown_sweep.next_target() else {
        return false;
    };
    let shutdown_region = shutdown_target.region();
    let Ok(mut shutdown_flight) = shutdown_target.start_flight(
        sent,
        ProbeStart {
            transport,
            buffer: spare,
        },
    ) else {
        return false;
    };

    shutdown_flight.begin_drain();
    if !shutdown_flight.is_draining() {
        return false;
    }
    let Poll::Ready(cancelled) = shutdown_flight.poll_complete(&mut cx) else {
        return false;
    };
    if cancelled.outcome() != TransferOutcome::Cancelled || cancelled.region() != shutdown_region {
        return false;
    }

    let (transport, cancelled_sent, cancelled_spare, settlement) = cancelled.into_parts();
    if shutdown_sweep.settle(settlement) != Ok(TransferOutcome::Cancelled)
        || !shutdown_sweep.is_poisoned()
    {
        return false;
    }
    let Ok((aborted, shutdown_snapshot)) = shutdown_sweep.abort() else {
        return false;
    };
    if demand.finish_failed(aborted, Tick(1)).is_err() {
        return false;
    }

    demand.is_dirty()
        && demand.sweeping().is_none()
        && demand.full_repaint_required()
        && transport.sequence == 2
        && transport.written_region == Some(shutdown_region)
        && cancelled_sent == [0xc3]
        && cancelled_spare == [0x5a]
        && shutdown_snapshot == 0x3c
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
