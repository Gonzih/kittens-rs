//! K2R-0A transfer boundary: a conditionally outer-`Unpin` in-flight
//! adapter over a waker-registering completion boundary.
//!
//! Mechanism verdict (K2R0A-LOG, external engineering contribution): on the
//! anchor board this boundary is implemented by a profile-owned SPI2
//! `TransferDone` ISR and critical-section waker slot — candidate C's
//! completion mechanism carried in candidate A′'s `Unpin` shape. The
//! generic carrier is `Unpin` exactly when its transfer and spare are; the
//! borrowing HAL completion future is not used at all.
//!
//! Contract obligations came out of review corrections and are load-bearing:
//!
//! - implementations MUST register-then-recheck in `poll_done`; the
//!   check-then-register order has a lost-wake race (a completion between
//!   the check and the registration wakes nobody, forever). The test suite
//!   contains a deliberately broken check-then-register model proving the
//!   adversarial oracle catches this;
//! - `cancel` MUST wake a registered waker: cancellation is progress, and
//!   hardware may produce no further interrupt to do it for us;
//! - recovery is the **sole outcome authority**: `poll_done` reports only
//!   settlement (`Poll<()>`), `recover` reports how;
//! - the spare buffer is carried through the in-flight state and returned at
//!   settlement, per SPEC section 7's resource-recovery criterion;
//! - sealing this trait to reviewed integrations is a pre-freeze obligation
//!   recorded in `K2R0A-LOG.md`; during the experiment it stays open so
//!   probes and models can implement it.

use core::task::{Context, Poll};

use crate::geometry::Region;
use crate::sweep::{StripeTarget, StripeWritten};

/// An owned, in-flight region transfer at the HAL boundary.
pub trait OwnedTransfer: Sized {
    /// The transport (bus/display handle) consumed by the transfer.
    type Transport;
    /// The pixel buffer consumed by the transfer.
    type Buffer;

    /// Polls for settlement, registering the current waker when pending.
    ///
    /// Register-then-recheck is mandatory (see module docs). Once settled,
    /// stays `Ready`. Reports only *that* the transfer settled; how it
    /// settled is [`OwnedTransfer::recover`]'s answer alone.
    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()>;

    /// Requests cancellation. Idempotent. Settles the transfer (possibly
    /// immediately) and MUST wake any registered waker — cancellation is
    /// progress and hardware may never interrupt again.
    fn cancel(&mut self);

    /// Consumes a settled transfer, returning the transport, the sent
    /// buffer, and the settlement outcome.
    fn recover(self) -> Recovered<Self::Transport, Self::Buffer>;
}

/// How an in-flight transfer settled. Produced only by
/// [`OwnedTransfer::recover`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransferOutcome {
    /// Every byte of the region was written by the transport's own
    /// completion definition. Says nothing about bus idleness or physical
    /// presentation.
    Completed,
    /// Cancellation settled the transfer first (its observation is the
    /// linearization point; a physical completion racing it is
    /// conservatively classified `Cancelled`). Region content is undefined;
    /// SPEC 5.3 forces a full repaint.
    Cancelled,
    /// The integration's reviewed fault source settled the transfer. The
    /// esp-hal SPI2 adapter has no post-start fault source and never
    /// produces this; it exists for boundaries that do (and for the model).
    Failed,
}

/// Everything an in-flight transfer consumed, returned at settlement.
#[derive(Debug)]
pub struct Recovered<T, B> {
    /// The transport, ready for the next transfer.
    pub transport: T,
    /// The buffer that was sent (stale scratch now).
    pub buffer: B,
    /// How the transfer settled.
    pub outcome: TransferOutcome,
}

/// Settlement of the full in-flight state. Fields are **private** (round-2
/// finding 4): safe code can neither construct a settlement nor rewrite
/// its outcome, so the coverage proof chain starts only at a real,
/// settled transfer.
#[derive(Debug)]
pub struct Settled<T, B, S> {
    transport: T,
    buffer: B,
    spare: S,
    outcome: TransferOutcome,
    /// Present exactly until the witness is minted: single-use.
    target: Option<StripeTarget>,
}

impl<T, B, S> Settled<T, B, S> {
    /// How the transfer settled.
    pub const fn outcome(&self) -> TransferOutcome {
        self.outcome
    }

    /// The region this settlement targeted while its single-use target is
    /// still present. Returns an empty region after
    /// [`Settled::stripe_written`] consumes that target.
    pub fn region(&self) -> Region {
        self.target.as_ref().map_or(
            Region {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            StripeTarget::region,
        )
    }

    /// Mints the coverage witness: **at most once**, and **only** for a
    /// `Completed` settlement — the mint consumes the settlement's target,
    /// so a second call returns `None` and a cancelled/failed settlement
    /// never mints (round-2 finding 4's single-use requirement).
    pub fn stripe_written(&mut self) -> Option<StripeWritten> {
        match self.outcome {
            TransferOutcome::Completed => self.target.take().map(|target| StripeWritten {
                demand_id: target.demand_id,
                epoch: target.epoch,
                region: target.region,
            }),
            TransferOutcome::Cancelled | TransferOutcome::Failed => None,
        }
    }

    /// Consumes the settlement, returning every resource.
    pub fn into_resources(self) -> (T, B, S) {
        (self.transport, self.buffer, self.spare)
    }
}

/// The in-flight adapter: conditionally `Unpin`, `&mut`-polled, drivable to
/// settlement, never resource-losing on the driven path. It implements
/// `Unpin` exactly when `X: OwnedTransfer + Unpin` and `S: Unpin`; the
/// associated transport and buffer types need no `Unpin` bound because
/// they are not stored separately in flight. Owns the transfer *and* the
/// spare buffer, which stays independently writable during the flight
/// (SPEC 6.2's transfer boundary).
///
/// ```text
/// InFlight ── poll_complete Ready ─────────────▶ Settled { .., Completed/Failed }
/// InFlight ── begin_drain ─▶ draining ── poll_complete Ready ─▶ Settled { .., Cancelled/Completed }
/// InFlight ── drop ─▶ resources lost (documented non-returning boundary)
/// ```
#[derive(Debug)]
pub struct InFlight<X: OwnedTransfer, S> {
    transfer: Option<X>,
    spare: Option<S>,
    draining: bool,
    target: Option<StripeTarget>,
}

impl<X: OwnedTransfer, S> InFlight<X, S> {
    /// Wraps a started transfer together with the spare buffer that remains
    /// writable during the flight, bound to the unforgeable stripe target
    /// minted by [`crate::sweep::Sweep::next_target`] — the transfer and
    /// its claimed identity can no longer be paired independently
    /// (round-2 finding 4).
    pub const fn new(transfer: X, spare: S, target: StripeTarget) -> Self {
        Self {
            transfer: Some(transfer),
            spare: Some(spare),
            draining: false,
            target: Some(target),
        }
    }

    /// The spare buffer, writable while the transfer is in flight.
    pub const fn spare_mut(&mut self) -> Option<&mut S> {
        self.spare.as_mut()
    }

    /// Requests cancellation; the transfer settles through
    /// [`InFlight::poll_complete`], the only path that returns resources.
    /// Idempotent.
    pub fn begin_drain(&mut self) {
        if let Some(transfer) = self.transfer.as_mut() {
            if !self.draining {
                self.draining = true;
                transfer.cancel();
            }
        }
    }

    /// Whether a drain was requested and the transfer has not yet settled.
    pub const fn is_draining(&self) -> bool {
        self.draining && self.transfer.is_some()
    }

    /// Polls the transfer to settlement. `Ready` consumes the in-flight
    /// state and returns every resource — transport, sent buffer, and
    /// spare. A spent adapter polls `Pending` forever without registering a
    /// wake.
    ///
    /// # Panics
    ///
    /// Never in practice: the internal takes follow a checked presence test
    /// on the same values.
    pub fn poll_complete(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Settled<X::Transport, X::Buffer, S>> {
        let Some(transfer) = self.transfer.as_mut() else {
            return Poll::Pending;
        };
        match transfer.poll_done(cx) {
            Poll::Ready(()) => {
                let transfer = self.transfer.take().expect("transfer present");
                let spare = self.spare.take().expect("spare present until settlement");
                let target = self.target.take().expect("target present until settlement");
                let recovered = transfer.recover();
                Poll::Ready(Settled {
                    transport: recovered.transport,
                    buffer: recovered.buffer,
                    spare,
                    outcome: recovered.outcome,
                    target: Some(target),
                })
            }
            Poll::Pending => Poll::Pending,
        }
    }

    /// Whether the transfer has settled and been recovered.
    pub const fn is_spent(&self) -> bool {
        self.transfer.is_none()
    }
}

// A′ premise: address-sensitive state lives behind the integration's own
// boundary (interrupt-registered statics), never in the value the reactor
// stores.
impl<X: OwnedTransfer + Unpin, S: Unpin> Unpin for InFlight<X, S> {}
