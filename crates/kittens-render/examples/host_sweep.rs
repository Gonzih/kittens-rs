//! Canonical host-side walk through a complete written-frame lifecycle and
//! an accepted-flight shutdown lifecycle.
//!
//! The transport below is deliberately small and synchronous, but it uses
//! the same ownership and proof boundaries as a hardware integration: the
//! transfer owns the transport and sent buffer,
//! [`InFlight`](kittens_render::transfer::InFlight) owns the spare
//! and an unforgeable target, and every settlement must reconcile with the
//! sweep (completion advances; failure or cancellation poisons).

use std::task::{Context, Poll, Waker};

use kittens_render::demand::{FrameDemand, Tick, WrittenDisposition};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::{StripeTarget, Sweep, SweepPlan};
use kittens_render::transfer::{FlightStarter, OwnedTransfer, Recovered, TransferOutcome};

const STRIPE_HEIGHT: u16 = 112;

#[derive(Debug)]
struct FrameSnapshot {
    scene: &'static str,
    tone: u8,
}

#[derive(Clone, Copy, Debug)]
struct RenderedStripe {
    epoch: u64,
    region: Region,
    digest: u32,
}

#[derive(Debug)]
struct StripeBuffer {
    name: &'static str,
    rendered: Option<RenderedStripe>,
}

impl StripeBuffer {
    /// Rendering reads the sweep-owned snapshot through a shared reference,
    /// so every stripe is derived from the same immutable scene state.
    fn render(&mut self, snapshot: &FrameSnapshot, target: &StripeTarget) {
        let region = target.region();
        let digest = u32::from(snapshot.tone)
            .wrapping_mul(16_777_619)
            .wrapping_add(u32::from(region.x))
            .wrapping_add(u32::from(region.y))
            .wrapping_add(u32::from(region.width))
            .wrapping_add(u32::from(region.height));
        self.rendered = Some(RenderedStripe {
            epoch: target.epoch().get(),
            region,
            digest,
        });
        println!(
            "  render: {} <- epoch {} y={}..{} digest={digest:#010x}",
            self.name,
            target.epoch().get(),
            region.y,
            region.y + region.height,
        );
    }

    /// These concrete buffers own disjoint `Option` storage, so this spare
    /// may be prepared while the sent buffer belongs to the transfer.
    /// Arbitrary generic buffer types do not get that disjointness guarantee.
    fn prepare_as_spare(&mut self) {
        self.rendered = None;
    }
}

#[derive(Debug, Default)]
struct HostTransport {
    written_stripes: u16,
}

impl HostTransport {
    /// Starting a transfer moves both the transport and rendered buffer into
    /// the owned completion boundary; neither can be reused before recovery.
    fn start(self, buffer: StripeBuffer, target_region: Region) -> HostTransfer {
        let rendered = buffer.rendered.expect("render the target before transfer");
        assert_eq!(rendered.region, target_region, "buffer matches its target");
        println!(
            "  start:  {} sends epoch {} digest={:#010x}",
            buffer.name, rendered.epoch, rendered.digest,
        );
        HostTransfer {
            transport: self,
            buffer,
            outcome: None,
        }
    }
}

#[derive(Debug)]
struct HostTransfer {
    transport: HostTransport,
    buffer: StripeBuffer,
    outcome: Option<TransferOutcome>,
}

impl OwnedTransfer for HostTransfer {
    type Transport = HostTransport;
    type Buffer = StripeBuffer;

    /// This host model completes on its first poll. A real pending
    /// integration must register its waker and then recheck completion.
    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        if self.outcome.is_none() {
            self.transport.written_stripes += 1;
            self.outcome = Some(TransferOutcome::Completed);
        }
        Poll::Ready(())
    }

    /// Cancellation remains an idempotent settlement path; the runnable
    /// shutdown cycle drains before the model's first completion poll.
    fn cancel(&mut self) {
        if self.outcome.is_none() {
            self.outcome = Some(TransferOutcome::Cancelled);
        }
    }

    /// Recovery is the sole outcome authority and returns every resource the
    /// transfer consumed; callers never infer success from polling alone.
    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.transport,
            buffer: self.buffer,
            outcome: self.outcome.expect("recover only after settlement"),
        }
    }
}

#[derive(Debug)]
struct HostStart {
    transport: HostTransport,
    buffer: StripeBuffer,
}

impl FlightStarter for HostStart {
    type Transfer = HostTransfer;
    type Error = core::convert::Infallible;

    /// Consuming this one-shot operation lets the target supply the only
    /// region that can reach the concrete transport start boundary.
    fn start(self, region: Region) -> Result<Self::Transfer, Self::Error> {
        Ok(self.transport.start(self.buffer, region))
    }
}

#[derive(Debug)]
struct HostResources {
    transport: HostTransport,
    ready: StripeBuffer,
    spare: StripeBuffer,
}

impl HostResources {
    /// Two named scratch buffers make the transfer/spare ownership rotation
    /// visible without allocating a full host framebuffer.
    fn new() -> Self {
        Self {
            transport: HostTransport::default(),
            ready: StripeBuffer {
                name: "buffer-a",
                rendered: None,
            },
            spare: StripeBuffer {
                name: "buffer-b",
                rendered: None,
            },
        }
    }
}

/// One stripe follows the complete proof chain: the sweep mints its target,
/// consuming the target invokes the starter with its exact region, settlement
/// returns one mandatory witness, and only reconciling that witness may
/// advance the sweep plan.
fn write_next_stripe(
    sweep: &mut Sweep<FrameSnapshot>,
    resources: HostResources,
    ordinal: u16,
    stripe_count: u16,
) -> HostResources {
    let target = sweep.next_target().expect("an unwritten stripe remains");
    let region = target.region();
    println!(
        "stripe {ordinal}/{stripe_count}: target epoch {} at ({}, {}) {}x{}",
        target.epoch().get(),
        region.x,
        region.y,
        region.width,
        region.height,
    );

    let HostResources {
        transport,
        mut ready,
        spare,
    } = resources;
    ready.render(sweep.snapshot(), &target);
    let mut in_flight = target
        .start_flight(
            spare,
            HostStart {
                transport,
                buffer: ready,
            },
        )
        .expect("the infallible host starter accepts the target");
    in_flight
        .spare_mut()
        .expect("spare remains available in flight")
        .prepare_as_spare();

    let mut context = Context::from_waker(Waker::noop());
    let settled = match in_flight.poll_complete(&mut context) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("the synchronous host model settles on its first poll"),
    };
    println!(
        "  settle: {:?} for y={}..{}",
        settled.outcome(),
        region.y,
        region.y + region.height,
    );

    let (transport, sent, next_ready, settlement) = settled.into_parts();
    assert_eq!(
        sweep
            .settle(settlement)
            .expect("the settlement matches the one outstanding target"),
        TransferOutcome::Completed,
    );
    println!("  settle: written coverage advanced");
    HostResources {
        transport,
        ready: next_ready,
        spare: sent,
    }
}

/// Demand is raised before `begin_sweep`; beginning is the sole eligibility
/// acknowledgement and moves one caller-frozen snapshot into the active
/// epoch.
fn begin_requested_frame(
    demand: &mut FrameDemand,
    now: Tick,
    snapshot: FrameSnapshot,
) -> Sweep<FrameSnapshot> {
    println!("demand: request frame for scene {:?}", snapshot.scene);
    demand.request();
    let sweep = demand
        .begin_sweep(now, snapshot)
        .expect("fresh demand is immediately eligible");
    println!(
        "sweep:  begin epoch {} (full_repaint={})",
        sweep.epoch().get(),
        sweep.full_repaint(),
    );
    sweep
}

/// `Sweep::finish` consumes only full coverage into `SweepWritten`; passing
/// that branded witness to `finish_written` is what settles demand and moves
/// the throttle milestone, without claiming physical presentation.
fn finish_written_frame(
    demand: &mut FrameDemand,
    sweep: Sweep<FrameSnapshot>,
    now: Tick,
) -> FrameSnapshot {
    let epoch = sweep.epoch();
    let (written, snapshot) = sweep
        .finish()
        .expect("every planned stripe was reconciled as written");
    println!("sweep:  finish epoch {} -> SweepWritten", epoch.get());
    let disposition = demand
        .finish_written(written, now)
        .expect("witness belongs to the active demand");
    assert_eq!(disposition, WrittenDisposition::Effective);
    println!(
        "demand: finish_written epoch {} -> {disposition:?}",
        epoch.get(),
    );
    snapshot
}

/// An accepted flight cannot be bypassed during shutdown: drain it, recover
/// every resource, reconcile its cancelled settlement (which poisons the
/// sweep), and only then abort and settle the active demand as failed.
fn drain_and_abort_frame(
    demand: &mut FrameDemand,
    mut sweep: Sweep<FrameSnapshot>,
    resources: HostResources,
    now: Tick,
) -> (HostResources, FrameSnapshot) {
    let target = sweep.next_target().expect("shutdown starts one flight");
    let region = target.region();
    let HostResources {
        transport,
        mut ready,
        spare,
    } = resources;
    let writes_before_drain = transport.written_stripes;
    let sent_name = ready.name;
    let spare_name = spare.name;

    ready.render(sweep.snapshot(), &target);
    let mut in_flight = target
        .start_flight(
            spare,
            HostStart {
                transport,
                buffer: ready,
            },
        )
        .expect("the shutdown flight is accepted before draining");
    in_flight
        .spare_mut()
        .expect("spare remains available in flight")
        .prepare_as_spare();

    println!("shutdown: begin_drain accepted flight at y={}", region.y);
    in_flight.begin_drain();
    assert!(in_flight.is_draining());
    let mut context = Context::from_waker(Waker::noop());
    let settled = match in_flight.poll_complete(&mut context) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("the cancelled host model settles on its first poll"),
    };
    assert_eq!(settled.outcome(), TransferOutcome::Cancelled);
    assert_eq!(settled.region(), region);

    let (transport, sent, next_ready, settlement) = settled.into_parts();
    assert_eq!(transport.written_stripes, writes_before_drain);
    assert_eq!(sent.name, sent_name);
    assert!(sent.rendered.is_some());
    assert_eq!(next_ready.name, spare_name);
    assert!(next_ready.rendered.is_none());
    assert_eq!(
        sweep
            .settle(settlement)
            .expect("the cancelled settlement matches the outstanding target"),
        TransferOutcome::Cancelled,
    );
    assert!(sweep.is_poisoned());
    println!("shutdown: cancelled settlement poisoned the sweep");

    let (aborted, snapshot) = sweep
        .abort()
        .expect("settlement cleared outstanding work before abort");
    demand
        .finish_failed(aborted, now)
        .expect("abort witness belongs to the active demand");
    assert!(demand.is_dirty());
    assert!(demand.sweeping().is_none());
    assert!(demand.full_repaint_required());
    println!("shutdown: abort succeeded; demand retained for full repaint");

    (
        HostResources {
            transport,
            ready: next_ready,
            spare: sent,
        },
        snapshot,
    )
}

/// Runs one full-panel frame and one accepted-flight shutdown over the admitted
/// Waveshare geometry, printing each owned lifecycle boundary in order.
fn main() {
    let geometry = PanelGeometry::WAVESHARE_18_V1;
    let panel = geometry.panel();
    let plan = SweepPlan::for_panel(geometry, STRIPE_HEIGHT).expect("anchor plan is valid");
    let stripe_count = plan.stripe_count();
    println!(
        "panel:  Waveshare 1.8 V1 {}x{}, {stripe_count} stripes",
        panel.width, panel.height,
    );

    let mut demand = FrameDemand::new(0, plan);
    let snapshot = FrameSnapshot {
        scene: "hello kittens",
        tone: 0x5a,
    };
    let mut sweep = begin_requested_frame(&mut demand, Tick(0), snapshot);
    let mut resources = HostResources::new();

    for index in 0..stripe_count {
        resources = write_next_stripe(&mut sweep, resources, index + 1, stripe_count);
    }

    let snapshot = finish_written_frame(
        &mut demand,
        sweep,
        Tick(u64::from(resources.transport.written_stripes)),
    );
    println!(
        "frame:  {:?} returned; {} stripes written; both buffers recovered",
        snapshot.scene, resources.transport.written_stripes,
    );

    let shutdown_tick = Tick(u64::from(resources.transport.written_stripes));
    let shutdown_sweep = begin_requested_frame(
        &mut demand,
        shutdown_tick,
        FrameSnapshot {
            scene: "shutdown recovery",
            tone: 0xa5,
        },
    );
    let writes_before_drain = resources.transport.written_stripes;
    let (resources, shutdown_snapshot) =
        drain_and_abort_frame(&mut demand, shutdown_sweep, resources, shutdown_tick);
    assert_eq!(resources.transport.written_stripes, writes_before_drain);
    assert_ne!(resources.ready.name, resources.spare.name);
    println!(
        "shutdown: {:?} returned; transport and buffers {} / {} recovered",
        shutdown_snapshot.scene, resources.ready.name, resources.spare.name,
    );
}
