//! Negative control: target/start pairing is structural under sealed
//! integrations, but the experiment-open trait still admits a starter that
//! returns a prestarted wrong-region transfer or starts and then rejects.
//! Sealing and integration review own both honesty obligations.

use std::cell::Cell;
use std::rc::Rc;
use std::task::{Context, Poll};

use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::geometry::{PanelGeometry, Region};
use kittens_render::sweep::SweepPlan;
use kittens_render::transfer::{
    FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome,
};

#[derive(Debug)]
struct NoopTransfer {
    started_region: Region,
}

impl OwnedTransfer for NoopTransfer {
    type Transport = ();
    type Buffer = ();

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }

    fn cancel(&mut self) {}

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        let _physical_region = self.started_region;
        Recovered {
            transport: (),
            buffer: (),
            outcome: TransferOutcome::Completed,
        }
    }
}

struct PrestartedWrongRegion {
    transfer: NoopTransfer,
}

impl FlightStarter for PrestartedWrongRegion {
    type Transfer = NoopTransfer;
    type Error = ();

    fn start(
        self,
        _required_region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        Ok(self.transfer)
    }
}

#[derive(Debug)]
struct RejectedAfterStart {
    physical_write_still_live: Rc<Cell<bool>>,
}

struct StartThenReject {
    physical_write_still_live: Rc<Cell<bool>>,
}

impl FlightStarter for StartThenReject {
    type Transfer = NoopTransfer;
    type Error = RejectedAfterStart;

    fn start(
        self,
        _required_region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        self.physical_write_still_live.set(true);
        Err(RejectedAfterStart {
            physical_write_still_live: self.physical_write_still_live,
        })
    }
}

fn mint_target() -> (
    kittens_render::sweep::Sweep<()>,
    kittens_render::sweep::StripeTarget,
) {
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 56).expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand.begin_sweep(Tick(0), ()).expect("sweep");
    let target = sweep.next_target().expect("target");
    (sweep, target)
}

fn main() {
    let (wrong_region_sweep, target) = mint_target();
    let required_region = target.region();
    let wrong_region = Region {
        y: required_region.y + required_region.height,
        ..required_region
    };
    assert_ne!(wrong_region, required_region);
    let wrong_region_flight = target
        .start_flight(
            (),
            PrestartedWrongRegion {
                transfer: NoopTransfer {
                    started_region: wrong_region,
                },
            },
        )
        .expect("the dishonest starter accepts its prestarted transfer");

    let (rejected_sweep, target) = mint_target();
    let physical_write_still_live = Rc::new(Cell::new(false));
    let rejection = match target.start_flight(
        (),
        StartThenReject {
            physical_write_still_live: Rc::clone(&physical_write_still_live),
        },
    ) {
        Ok(_) => panic!("dishonest starter rejects"),
        Err(rejection) => rejection,
    };
    let (error, (), _target) = rejection.into_parts();
    assert!(error.physical_write_still_live.get());
    assert!(physical_write_still_live.get());

    let _still_compiles = (
        wrong_region_flight,
        wrong_region_sweep,
        rejected_sweep,
        physical_write_still_live,
    );
}
