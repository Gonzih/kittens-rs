use core::future::Future;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::task::{Context, Poll};

use kittens::source::OptionalInlineOneShot;

struct PinnedFuture {
    _pin: PhantomPinned,
}

impl Future for PinnedFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

fn main() {
    let _ = OptionalInlineOneShot::from_future(PinnedFuture {
        _pin: PhantomPinned,
    });
}
