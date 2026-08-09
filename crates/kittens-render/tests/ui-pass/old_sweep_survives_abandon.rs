//! Negative control: `abandon_active` terminally rejects the old epoch in
//! the demand machine, but ordinary Rust cannot invalidate a retained sweep.
//! It can still mint and start old work after the replacement begins. This
//! explicit escape is bounded only when the caller drops the flight and the
//! reviewed adapter synchronously cancels/disarms in `Drop`; the type system
//! cannot force a caller to drop rather than retain or drive it.

use std::cell::Cell;
use std::convert::Infallible;
use std::rc::Rc;
use std::task::{Context, Poll};

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{
    FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome,
};

struct DropCancelledTransfer {
    region: Region,
    cancelled: Rc<Cell<bool>>,
    outcome: Option<TransferOutcome>,
}

impl OwnedTransfer for DropCancelledTransfer {
    type Transport = Region;
    type Buffer = ();

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        self.outcome.get_or_insert(TransferOutcome::Completed);
        Poll::Ready(())
    }

    fn cancel(&mut self) {
        if self.outcome.is_none() {
            self.cancelled.set(true);
            self.outcome = Some(TransferOutcome::Cancelled);
        }
    }

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.region,
            buffer: (),
            outcome: self.outcome.expect("poll or cancel before recovery"),
        }
    }
}

impl Drop for DropCancelledTransfer {
    fn drop(&mut self) {
        self.cancel();
    }
}

struct LateStart {
    cancelled: Rc<Cell<bool>>,
}

impl FlightStarter for LateStart {
    type Transfer = DropCancelledTransfer;
    type Error = Infallible;

    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        Ok(DropCancelledTransfer {
            region,
            cancelled: self.cancelled,
            outcome: None,
        })
    }
}

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut old_sweep = demand.begin_sweep(Tick(0), 0_u8).expect("old sweep");

    demand.abandon_active();
    let replacement = demand
        .begin_sweep(Tick(0), 1_u8)
        .expect("replacement sweep");
    let stale_target = old_sweep
        .next_target()
        .expect("retained old sweep can still mint after replacement");
    let cancelled = Rc::new(Cell::new(false));
    let stale_flight = stale_target
        .start_flight(
            (),
            LateStart {
                cancelled: Rc::clone(&cancelled),
            },
        )
        .expect("old work can still start through an open integration");

    drop(stale_flight);
    assert!(cancelled.get(), "adapter Drop bounded the stale operation");
    let Err(old_sweep) = old_sweep.abort() else {
        panic!("drop returns no settlement, so the old sweep remains outstanding")
    };

    let (aborted, _snapshot) = replacement
        .abort()
        .expect("replacement has no outstanding target");
    demand.finish_failed(aborted, Tick(0)).expect("replacement");
    drop(old_sweep);
}
