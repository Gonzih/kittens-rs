//! Negative control: ownership separates the sent-buffer and spare values,
//! but their types may still share safe interior-mutable backing storage.

use std::cell::Cell;
use std::convert::Infallible;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{FlightStarter, OwnedTransfer, Recovered, TransferOutcome};

struct AliasTransfer {
    region: Region,
    buffer: Rc<Cell<u8>>,
}

impl OwnedTransfer for AliasTransfer {
    type Transport = Region;
    type Buffer = Rc<Cell<u8>>;

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }

    fn cancel(&mut self) {}

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.region,
            buffer: self.buffer,
            outcome: TransferOutcome::Completed,
        }
    }
}

struct AliasStart {
    sent: Rc<Cell<u8>>,
}

impl FlightStarter for AliasStart {
    type Transfer = AliasTransfer;
    type Error = Infallible;

    fn start(self, region: Region) -> Result<Self::Transfer, Self::Error> {
        Ok(AliasTransfer {
            region,
            buffer: self.sent,
        })
    }
}

fn main() {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");
    let target = sweep.next_target().expect("target");

    let shared = Rc::new(Cell::new(1));
    let sent = Rc::clone(&shared);
    let spare = Rc::clone(&shared);
    let mut flight = target
        .start_flight(spare, AliasStart { sent })
        .expect("infallible start");

    flight.spare_mut().expect("spare").set(7);
    let mut cx = Context::from_waker(Waker::noop());
    let Poll::Ready(settled) = flight.poll_complete(&mut cx) else {
        panic!("immediate model settles");
    };
    let (_region, sent, spare, settlement) = settled.into_parts();
    assert_eq!(sent.get(), 7, "the sent value observes spare mutation");
    assert_eq!(spare.get(), 7);
    sweep.settle(settlement).expect("own settlement");

    let (aborted, ()) = sweep.abort().expect("settled target permits abort");
    demand.finish_failed(aborted, Tick(0)).expect("active");
}
