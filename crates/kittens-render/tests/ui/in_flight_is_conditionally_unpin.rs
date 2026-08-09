use core::marker::PhantomPinned;
use core::task::{Context, Poll};

use kittens_render::transfer::{InFlight, OwnedTransfer, Recovered, TransferOutcome};

struct AddressSensitive(PhantomPinned);

impl OwnedTransfer for AddressSensitive {
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

fn assert_unpin<T: Unpin>() {}

fn main() {
    assert_unpin::<InFlight<AddressSensitive, ()>>();
}
