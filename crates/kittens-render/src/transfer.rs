//! K2R-0A candidate A′: an outer-`Unpin` in-flight adapter over a transfer
//! boundary that exposes waker-registering completion polling instead of a
//! borrowing completion future.
//!
//! The review established (SPEC review finding 2) that the HAL's borrowing
//! `wait_for_done(&mut transfer)` future cannot be named as a stable,
//! no-alloc associated type. Candidate A′ therefore moves the boundary: the
//! integration implements [`OwnedTransfer::poll_done`] — `&mut self`,
//! waker-registering, no future object — and the adapter stays `Unpin` and
//! `&mut`-polled, which the current kernel source contract can admit.
//!
//! Whether `poll_done` is honestly implementable over the *real* esp-hal
//! transfer (via `is_done` plus interrupt-driven waker registration, or only
//! via candidate C's hand-built interrupt state) is exactly the open half of
//! the experiment; `K2R0A-LOG.md` tracks it, and the exact-HAL compile probe
//! is gated on the Xtensa toolchain.

use core::task::{Context, Poll};

/// An owned, in-flight region transfer at the HAL boundary.
///
/// Contract, per SPEC section 5:
///
/// - `poll_done` registers the current waker when pending and MUST NOT
///   self-wake while no progress is possible (busy-poll rejection is a
///   K2R-0A pass criterion, asserted by wake-count oracles);
/// - completion state is level-like: once done, `poll_done` stays `Ready`
///   and `recover` is legal;
/// - `cancel` requests cancellation; the transfer still completes (as
///   cancelled) through `poll_done`, so recovery is always driven to
///   settlement — this is the explicit cancel-and-drain path. Ordinary drop
///   of the transfer is the documented non-returning boundary;
/// - `recover` consumes the settled transfer and returns every resource.
pub trait OwnedTransfer: Sized {
    /// The transport (bus/display handle) consumed by the transfer.
    type Transport;
    /// The pixel buffer consumed by the transfer.
    type Buffer;

    /// Polls for settlement, registering the waker when pending.
    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<TransferOutcome>;

    /// Requests cancellation. Idempotent. The transfer settles (possibly
    /// immediately) as [`TransferOutcome::Cancelled`] unless it had already
    /// completed.
    fn cancel(&mut self);

    /// Consumes a settled transfer, returning the transport and buffer.
    ///
    /// Calling this before `poll_done` returned `Ready` is a contract
    /// violation the integration MUST make impossible or reject; the model
    /// transport panics in tests to surface it.
    fn recover(self) -> Recovered<Self::Transport, Self::Buffer>;
}

/// How an in-flight transfer settled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransferOutcome {
    /// Every byte of the region was written and flushed by the transport's
    /// own definition of completion. This is `StripeWritten` material — it
    /// says nothing about bus idleness or physical presentation.
    Completed,
    /// Cancellation settled the transfer before completion. The panel region
    /// content is undefined; SPEC 5.3 forces a full repaint.
    Cancelled,
    /// The transport failed. Panel state for the region is undefined; SPEC
    /// 5.3 forces a full repaint.
    Failed,
}

/// Everything an in-flight transfer consumed, returned at settlement.
#[derive(Debug)]
pub struct Recovered<T, B> {
    /// The transport, ready for the next transfer.
    pub transport: T,
    /// The buffer that was sent (its content is stale scratch now).
    pub buffer: B,
    /// How the transfer settled.
    pub outcome: TransferOutcome,
}

/// Candidate A′ in-flight adapter: `Unpin`, `&mut`-polled, drivable to
/// settlement, never resource-losing on the driven path.
///
/// State machine (SPEC 6, corrected per review findings 3/6):
///
/// ```text
/// InFlight ── poll_complete Ready ──▶ (Recovered { .., Completed/Failed })
/// InFlight ── begin_drain ──▶ Draining ── poll_complete Ready ──▶ (Recovered { .., Cancelled/Completed/Failed })
/// InFlight/Draining ── drop ──▶ resources lost (documented non-returning boundary)
/// ```
#[derive(Debug)]
pub struct InFlight<X: OwnedTransfer> {
    transfer: Option<X>,
    draining: bool,
}

impl<X: OwnedTransfer> InFlight<X> {
    /// Wraps a started transfer.
    pub const fn new(transfer: X) -> Self {
        Self {
            transfer: Some(transfer),
            draining: false,
        }
    }

    /// Requests cancellation; the transfer still settles through
    /// [`InFlight::poll_complete`], which is the only path that returns the
    /// resources. Idempotent.
    pub fn begin_drain(&mut self) {
        if let Some(transfer) = self.transfer.as_mut() {
            if !self.draining {
                self.draining = true;
                transfer.cancel();
            }
        }
    }

    /// Whether a drain has been requested and the transfer has not yet
    /// settled.
    pub const fn is_draining(&self) -> bool {
        self.draining
    }

    /// Polls the transfer to settlement. `Ready` consumes the in-flight
    /// state and returns every resource; afterwards the adapter is spent and
    /// further polls return `Pending` forever without registering a wake
    /// (the caller owns moving on).
    pub fn poll_complete(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Recovered<X::Transport, X::Buffer>> {
        let Some(transfer) = self.transfer.as_mut() else {
            return Poll::Pending;
        };
        match transfer.poll_done(cx) {
            Poll::Ready(_outcome) => {
                let transfer = self.transfer.take().expect("transfer present");
                Poll::Ready(transfer.recover())
            }
            Poll::Pending => Poll::Pending,
        }
    }

    /// Whether the transfer has settled and been recovered.
    pub const fn is_spent(&self) -> bool {
        self.transfer.is_none()
    }
}

// The adapter is Unpin whenever the transfer value itself is movable, which
// is the A′ premise: the integration keeps any address-sensitive state
// behind its own boundary (e.g. interrupt-registered statics), not in the
// value the reactor stores.
impl<X: OwnedTransfer + Unpin> Unpin for InFlight<X> {}
