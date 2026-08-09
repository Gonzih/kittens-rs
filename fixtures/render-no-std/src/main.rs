#![cfg_attr(any(target_os = "none", target_arch = "wasm32"), no_std)]
#![cfg_attr(any(target_os = "none", target_arch = "wasm32"), no_main)]

use core::convert::Infallible;
use core::future::Future;
#[cfg(any(target_os = "none", target_arch = "wasm32"))]
use core::panic::PanicInfo;
use core::task::{Context, Poll, Waker};

use kittens::reactor::Control;
use kittens::source::OptionalInlineOneShot;
use kittens_render::demand::{FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::{Sweep, SweepPlan};
use kittens_render::transfer::{
    FlightStarter, InFlight, OwnedTransfer, Recovered, Settled, StartPermit, TransferOutcome,
};

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
    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        self.transport.start_region(region, self.buffer)
    }
}

type ProbeFlight = InFlight<ProbeTransfer, [u8; 1]>;
type ProbeSettlement = Settled<ProbeTransport, [u8; 1], [u8; 1]>;

struct Sources {
    completion: OptionalInlineOneShot<ProbeFlight>,
}

struct RenderRun {
    demand: FrameDemand,
    sweep: Option<Sweep<u8>>,
    successful_stripes: u16,
    shutdown_phase: bool,
    drain_requested: bool,
    invalid: bool,
}

impl RenderRun {
    fn start() -> Option<(Self, Sources)> {
        // Two stripes force the successful path to rearm the same carrier
        // before the later shutdown flight exercises its borrowed drain hook.
        let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 224).ok()?;
        let mut demand = FrameDemand::new(0, plan);
        demand.request();
        let mut sweep = demand.begin_sweep(Tick(0), 0xa5_u8)?;
        let target = sweep.next_target()?;
        let flight = target
            .start_flight(
                [0xc3],
                ProbeStart {
                    transport: ProbeTransport {
                        sequence: 0,
                        written_region: None,
                    },
                    buffer: [0x5a],
                },
            )
            .ok()?;

        Some((
            Self {
                demand,
                sweep: Some(sweep),
                successful_stripes: 0,
                shutdown_phase: false,
                drain_requested: false,
                invalid: false,
            },
            Sources {
                completion: OptionalInlineOneShot::from_future(flight),
            },
        ))
    }

    fn handle_settlement(
        &mut self,
        completion: &mut OptionalInlineOneShot<ProbeFlight>,
        settled: ProbeSettlement,
    ) -> Control<bool> {
        // The carrier removes the completed future before this handler gets
        // its owned settlement, which is what makes immediate rearm sound.
        if self.invalid || !completion.is_dormant() {
            return Control::Stop(false);
        }
        let region = settled.region();
        let outcome = settled.outcome();
        let (transport, sent, spare, settlement) = settled.into_parts();
        let Some(mut sweep) = self.sweep.take() else {
            return Control::Stop(false);
        };

        if transport.written_region != Some(region) || sweep.settle(settlement) != Ok(outcome) {
            return Control::Stop(false);
        }

        if self.shutdown_phase {
            if !self.drain_requested
                || outcome != TransferOutcome::Cancelled
                || !sweep.is_poisoned()
            {
                return Control::Stop(false);
            }
            let Ok((aborted, snapshot)) = sweep.abort() else {
                return Control::Stop(false);
            };
            if self.demand.finish_failed(aborted, Tick(1)).is_err() {
                return Control::Stop(false);
            }

            return Control::Stop(
                snapshot == 0x3c
                    && self.demand.is_dirty()
                    && self.demand.sweeping().is_none()
                    && self.demand.full_repaint_required()
                    && transport.sequence == 3
                    && sent == [0x5a]
                    && spare == [0xc3],
            );
        }

        if outcome != TransferOutcome::Completed {
            return Control::Stop(false);
        }
        self.successful_stripes += 1;

        if let Some(target) = sweep.next_target() {
            let Ok(flight) = target.start_flight(
                sent,
                ProbeStart {
                    transport,
                    buffer: spare,
                },
            ) else {
                return Control::Stop(false);
            };
            self.sweep = Some(sweep);
            return match completion.arm(flight) {
                Ok(()) => Control::Continue,
                Err(_) => Control::Stop(false),
            };
        }

        let Ok((written, snapshot)) = sweep.finish() else {
            return Control::Stop(false);
        };
        if self.demand.finish_written(written, Tick(1)) != Ok(WrittenDisposition::Effective)
            || snapshot != 0xa5
            || self.successful_stripes != 2
            || self.demand.is_dirty()
            || self.demand.sweeping().is_some()
            || transport.sequence != 2
        {
            return Control::Stop(false);
        }

        // Shutdown owns a real accepted flight. It rearms the same carrier;
        // `before_poll` below requests cancellation through `future_mut`, and
        // the reactor must still deliver the cancelled settlement before stop.
        self.demand.request();
        let Some(mut shutdown_sweep) = self.demand.begin_sweep(Tick(1), 0x3c_u8) else {
            return Control::Stop(false);
        };
        let Some(target) = shutdown_sweep.next_target() else {
            return Control::Stop(false);
        };
        let Ok(flight) = target.start_flight(
            sent,
            ProbeStart {
                transport,
                buffer: spare,
            },
        ) else {
            return Control::Stop(false);
        };
        self.sweep = Some(shutdown_sweep);
        self.shutdown_phase = true;
        match completion.arm(flight) {
            Ok(()) => Control::Continue,
            Err(_) => Control::Stop(false),
        }
    }
}

/// Exercises the public render proof chain from a separate crate through a
/// generated reactor so the portable gates link real source admission,
/// successful settlement/rearm, and graceful shutdown drain.
async fn reactor_render_path(
    state: &mut RenderRun,
    sources: &mut Sources,
) -> Result<bool, Infallible> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [before_poll];
        }

        /// Once shutdown owns an accepted flight, cancellation must borrow
        /// that exact retained future and still settle through the source arm.
        before_poll {
            if state.shutdown_phase && !state.drain_requested {
                if let Some(flight) = sources.completion.future_mut() {
                    flight.begin_drain();
                    state.drain_requested = true;
                } else {
                    state.invalid = true;
                }
            }
            Ok(())
        }

        /// Completion owns every resource, so the handler can settle its
        /// sweep before rearming the same dormant carrier with the next flight.
        #[source(completion)]
        #[readiness(quiescent)]
        settled = sources.completion => {
            Ok(state.handle_settlement(&mut sources.completion, settled))
        }
    }
}

fn linked_render_path() -> bool {
    let Some((mut state, mut sources)) = RenderRun::start() else {
        return false;
    };
    let future = reactor_render_path(&mut state, &mut sources);
    let mut future = core::pin::pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    matches!(future.as_mut().poll(&mut cx), Poll::Ready(Ok(true)))
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(any(target_os = "none", target_arch = "wasm32"))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Keep the linked downstream proof-chain path live in the optimized
    // binary; this fixture is evidence about a consumer link, not only type
    // checking.
    let linked = linked_render_path();
    core::hint::black_box(linked);

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
fn main() {
    assert!(linked_render_path());
}
