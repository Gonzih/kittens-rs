//! Negative control: target/start pairing is structural, but an open adapter
//! can still ignore the supplied region. Sealing and integration review own
//! physical-write honesty.

use std::convert::Infallible;
use std::task::{Context, Poll};

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::PanelGeometry;
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{OwnedTransfer, Recovered, TransferOutcome};

struct NoopTransfer;

impl OwnedTransfer for NoopTransfer {
    type Transport = ();
    type Buffer = ();

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }

    fn cancel(&mut self) {}

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: (),
            buffer: (),
            outcome: TransferOutcome::Completed,
        }
    }
}

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");
    let target = sweep.next_target().expect("target");

    let flight = target
        .start_flight((), |_required_region| {
            Ok::<NoopTransfer, Infallible>(NoopTransfer)
        })
        .expect("infallible start");

    let _still_compiles = (flight, sweep, demand);
}
