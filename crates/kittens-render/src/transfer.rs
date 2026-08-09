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
//! - sealing [`OwnedTransfer`] and [`FlightStarter`] to reviewed integrations
//!   is a pre-freeze obligation recorded in `K2R0A-LOG.md`; during the
//!   experiment they stay open so probes and models can implement them. A
//!   dishonest experiment-phase starter can ignore its region, return a
//!   prestarted transfer, or start and then reject; that is the documented
//!   integration-honesty escape, not a structural guarantee. Safe external
//!   code still cannot invoke a starter directly: [`StartPermit`] is issued
//!   only inside [`StripeTarget::start_flight`].

use core::task::{Context, Poll};

use crate::geometry::Region;
use crate::sweep::{StripeSettlement, StripeTarget, StripeUnwritten, StripeWritten};

/// An owned, in-flight region transfer at the HAL boundary.
///
/// A reviewed implementation's `Drop` MUST synchronously cancel any pending
/// physical operation and disarm its completion registration. The trait is
/// intentionally open during the experiment, so that obligation becomes
/// structural only when integrations are reviewed and sealed at freeze.
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

/// Crate-issued authority for one [`FlightStarter::start`] dispatch.
///
/// The constructor and field are private, and this type deliberately does not
/// implement `Clone`. Its lifetime is tied to one
/// [`StripeTarget::start_flight`] call, so an experiment-phase starter cannot
/// return the received permit in its lifetime-independent error type for later
/// direct invocation. The type is public only because external integrations
/// must name it when implementing [`FlightStarter`].
pub struct StartPermit<'a> {
    _key: &'a mut (),
}

impl<'a> StartPermit<'a> {
    pub(crate) const fn new(key: &'a mut ()) -> Self {
        Self { _key: key }
    }
}

/// One operation-bound capability for starting a transfer at a crate-supplied
/// region.
///
/// This trait is deliberately open during the experiment and MUST be sealed
/// to reviewed integrations on the same pre-freeze schedule as
/// [`OwnedTransfer`]. Requiring a crate-issued [`StartPermit`] makes direct
/// safe invocation unavailable even while the trait remains open. Under sealed
/// integrations, consuming the starter and invoking it inside
/// [`StripeTarget::start_flight`] makes target/start pairing structural. While
/// it remains open, safe dishonest implementations can ignore `region`, return
/// a transfer started for another region, or start and then return `Err`; those
/// are explicit integration-honesty escapes in the same class as
/// `TouchReader`'s untorn-snapshot obligation.
pub trait FlightStarter: Sized {
    /// The accepted, already-started transfer.
    type Transfer: OwnedTransfer;
    /// Rejection plus every resource captured by this operation.
    type Error;

    /// Starts exactly `region` under one crate-issued dispatch permit,
    /// consuming the operation, permit, and operation resources.
    ///
    /// `Err` is acceptance-atomic by contract: no live transfer exists and no
    /// later physical write can result. An operation that may still complete
    /// MUST return its [`OwnedTransfer`] in `Ok`.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` only when no transfer was accepted; it owns every
    /// resource captured by this operation so the caller can recover them.
    fn start(self, region: Region, permit: StartPermit<'_>) -> Result<Self::Transfer, Self::Error>;
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
#[must_use = "dropping this value loses resources and leaves its sweep outstanding"]
#[derive(Debug)]
pub struct Settled<T, B, S> {
    transport: T,
    buffer: B,
    spare: S,
    outcome: TransferOutcome,
    target: StripeTarget,
}

impl<T, B, S> Settled<T, B, S> {
    /// How the transfer settled.
    pub const fn outcome(&self) -> TransferOutcome {
        self.outcome
    }

    /// The exact target region supplied to the [`FlightStarter`] that produced
    /// this transfer. Whether an experiment-phase implementation really wrote
    /// it remains a reviewed-integration obligation.
    pub const fn region(&self) -> Region {
        self.target.region()
    }

    /// Consumes the settlement and returns every resource together with the
    /// single-use witness that the cooperative path delivers to the transfer's
    /// owning sweep. Completion yields `Written`; cancellation or failure
    /// yields `Unwritten`. Rust prevents forging, relabeling, or duplicating
    /// that witness, but cannot force its delivery: dropping it or consuming
    /// it in a rejected wrong-sweep call is a published recovery escape. Drop
    /// the outstanding owner and use
    /// [`crate::demand::FrameDemand::abandon_active`] to retain demand and
    /// force a full repaint; use idle `invalidate` before replacement if stale
    /// physical work or external invalidation may overlap.
    pub fn into_parts(self) -> (T, B, S, StripeSettlement) {
        let settlement = match self.outcome {
            TransferOutcome::Completed => StripeSettlement::Written(StripeWritten {
                demand_id: self.target.demand_id,
                epoch: self.target.epoch,
                region: self.target.region,
            }),
            TransferOutcome::Cancelled | TransferOutcome::Failed => {
                StripeSettlement::Unwritten(StripeUnwritten {
                    demand_id: self.target.demand_id,
                    epoch: self.target.epoch,
                    region: self.target.region,
                    outcome: self.outcome,
                })
            }
        };
        (self.transport, self.buffer, self.spare, settlement)
    }
}

/// A target-driven start was reported rejected before any transfer was
/// accepted.
///
/// The error owns the starter-defined recovery value, the untouched spare,
/// and the same target, so callers can retry that exact position. The owning
/// sweep remains outstanding and cannot abort until an accepted retry settles;
/// callers that stop retrying must instead drop the target and sweep, then use
/// `FrameDemand::abandon_active`. Any transport or sent-buffer resources
/// captured by the starter must be returned inside `E`; while [`FlightStarter`]
/// remains open, safe Rust cannot enforce that acceptance-atomic integration
/// obligation.
#[derive(Debug)]
pub struct StartFlightError<E, S> {
    error: E,
    spare: S,
    target: StripeTarget,
}

impl<E, S> StartFlightError<E, S> {
    /// Returns the rejected start's error, spare, and original target.
    /// Consuming all three together makes the retry/resource-recovery decision
    /// explicit and prevents minting a second target for the position.
    pub fn into_parts(self) -> (E, S, StripeTarget) {
        (self.error, self.spare, self.target)
    }
}

impl StripeTarget {
    /// Starts the transfer from this target's exact region and, on success,
    /// moves the returned transfer, spare, and same target into flight.
    ///
    /// This is the only public path into [`InFlight`]. It issues the fresh
    /// [`StartPermit`] required by [`FlightStarter::start`], so safe callers
    /// cannot dispatch a starter without consuming a target. Target/start
    /// pairing is structural once [`FlightStarter`] is sealed to reviewed
    /// integrations. During the experiment, a dishonest implementation can
    /// still ignore the supplied region or return an independently started
    /// transfer.
    ///
    /// # Errors
    ///
    /// [`StartFlightError`] returns `E`, the spare, and the original target.
    /// Returning `Err` is an acceptance-atomic [`FlightStarter`] contract: no
    /// transfer was started and no later physical write can result from the
    /// attempt. A start that may still complete must return its
    /// [`OwnedTransfer`] in `Ok`, not `Err`.
    pub fn start_flight<F, S>(
        self,
        spare: S,
        starter: F,
    ) -> Result<InFlight<F::Transfer, S>, StartFlightError<F::Error, S>>
    where
        F: FlightStarter,
    {
        let mut permit_key = ();
        let permit = StartPermit::new(&mut permit_key);
        match starter.start(self.region, permit) {
            Ok(transfer) => Ok(InFlight::from_started(transfer, spare, self)),
            Err(error) => Err(StartFlightError {
                error,
                spare,
                target: self,
            }),
        }
    }
}

/// The in-flight adapter: conditionally `Unpin`, `&mut`-polled, drivable to
/// settlement, never resource-losing on the driven path. It implements
/// `Unpin` exactly when `X: OwnedTransfer + Unpin` and `S: Unpin`; the
/// associated transport and buffer types need no `Unpin` bound because
/// they are not stored separately in flight. Owns the transfer *and* the
/// spare buffer, whose owned value stays writable during the flight. The
/// generic types do not prove that sent and spare buffers have disjoint
/// backing storage: safe shared/interior-mutable aliases remain a documented
/// integration escape (SPEC 6.2). Ordinary `InFlight` drop returns no
/// resources or settlement witness; the reviewed `OwnedTransfer` adapter MUST
/// synchronously cancel the physical operation and disarm its registration in
/// its own `Drop` implementation, bounding that explicit escape.
///
/// ```text
/// InFlight ── poll_complete Ready ─────────────▶ Settled { .., Completed/Failed }
/// InFlight ── begin_drain ─▶ draining ── poll_complete Ready ─▶ Settled { .., Cancelled/Completed }
/// InFlight ── drop ─▶ adapter synchronously cancels/disarms; resources lost
/// ```
#[derive(Debug)]
pub struct InFlight<X: OwnedTransfer, S> {
    transfer: Option<X>,
    spare: Option<S>,
    draining: bool,
    target: Option<StripeTarget>,
}

impl<X: OwnedTransfer, S> InFlight<X, S> {
    pub(crate) const fn from_started(transfer: X, spare: S, target: StripeTarget) -> Self {
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
                    target,
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

#[cfg(test)]
mod tests {
    use core::task::Waker;

    use super::*;
    use crate::geometry::FrameEpoch;

    // This is a contract-conforming model used to observe `InFlight` itself.
    // Its behavior is test setup, not evidence that arbitrary implementations
    // of the deliberately open `OwnedTransfer` trait satisfy the contract.
    struct TestTransfer {
        outcome: Option<TransferOutcome>,
        waker: Option<Waker>,
    }

    impl TestTransfer {
        const fn pending() -> Self {
            Self {
                outcome: None,
                waker: None,
            }
        }
    }

    impl OwnedTransfer for TestTransfer {
        type Transport = u8;
        type Buffer = u8;

        fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
            // This deliberately uses the production integration order:
            // register first, then recheck settlement.
            self.waker = Some(cx.waker().clone());
            if self.outcome.is_some() {
                self.waker = None;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }

        fn cancel(&mut self) {
            if self.outcome.is_none() {
                self.outcome = Some(TransferOutcome::Cancelled);
            }
            if let Some(waker) = self.waker.take() {
                waker.wake();
            }
        }

        fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
            Recovered {
                transport: 11,
                buffer: 22,
                outcome: self.outcome.expect("test transfer settled"),
            }
        }
    }

    fn target() -> StripeTarget {
        StripeTarget {
            demand_id: 7,
            epoch: FrameEpoch(3),
            region: Region {
                x: 5,
                y: 9,
                width: 2,
                height: 1,
            },
        }
    }

    fn settlement_facts(settlement: StripeSettlement) -> (TransferOutcome, FrameEpoch, Region) {
        match settlement {
            StripeSettlement::Written(written) => (
                TransferOutcome::Completed,
                written.epoch(),
                written.region(),
            ),
            StripeSettlement::Unwritten(unwritten) => {
                (unwritten.outcome(), unwritten.epoch(), unwritten.region())
            }
        }
    }

    #[test]
    fn model_transfer_fixture_cancel_is_idempotent() {
        // `OwnedTransfer` remains an open integration contract. This oracle
        // validates the contract-conforming test model used below; it does not
        // claim that arbitrary external implementations are thereby checked.
        let mut transfer = TestTransfer::pending();
        transfer.cancel();
        transfer.cancel();
        assert_eq!(transfer.outcome, Some(TransferOutcome::Cancelled));
    }

    #[test]
    fn driven_flight_is_pending_ready_then_terminal() {
        let mut flight = InFlight::from_started(TestTransfer::pending(), 33, target());
        let mut cx = Context::from_waker(Waker::noop());
        assert!(flight.poll_complete(&mut cx).is_pending());

        flight.begin_drain();
        flight.begin_drain();
        assert!(flight.is_draining());
        let mut settled = None;
        let _ = flight
            .poll_complete(&mut cx)
            .map(|value| settled = Some(value));
        let settled = settled.expect("cancelled transfer must settle");
        assert_eq!(settled.outcome(), TransferOutcome::Cancelled);
        assert!(flight.is_spent());
        assert!(!flight.is_draining());
        flight.begin_drain();
        assert!(!flight.is_draining(), "draining a spent flight is inert");
        assert!(flight.spare_mut().is_none());
        assert!(flight.poll_complete(&mut cx).is_pending());

        let (transport, buffer, spare, settlement) = settled.into_parts();
        assert_eq!((transport, buffer, spare), (11, 22, 33));
        assert_eq!(
            settlement_facts(settlement),
            (TransferOutcome::Cancelled, FrameEpoch(3), target().region())
        );

        // The same concrete flight/settlement types must map real completion
        // to Written rather than sharing cancellation's Unwritten authority.
        let completed = TestTransfer {
            outcome: Some(TransferOutcome::Completed),
            waker: None,
        };
        let mut flight = InFlight::from_started(completed, 44, target());
        let mut settled = None;
        let _ = flight
            .poll_complete(&mut cx)
            .map(|value| settled = Some(value));
        let settled = settled.expect("completed transfer must settle");
        let (transport, buffer, spare, settlement) = settled.into_parts();
        assert_eq!((transport, buffer, spare), (11, 22, 44));
        assert_eq!(
            settlement_facts(settlement),
            (TransferOutcome::Completed, FrameEpoch(3), target().region())
        );
    }
}
