//! Positive half of the conditional-`Unpin` boundary: the carrier is
//! movable when both the owned transfer and spare are themselves `Unpin`.

use core::marker::PhantomPinned;
use core::task::{Context, Poll};

use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

struct MovableTransfer;

impl OwnedTransfer for MovableTransfer {
    type Transport = PhantomPinned;
    type Buffer = PhantomPinned;

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }

    fn cancel(&mut self) {}

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: PhantomPinned,
            buffer: PhantomPinned,
            outcome: TransferOutcome::Completed,
        }
    }
}

fn assert_unpin<T: Unpin>() {}

fn main() {
    assert_unpin::<InFlight<MovableTransfer, ()>>();
}
