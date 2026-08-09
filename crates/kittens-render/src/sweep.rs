//! The crate-owned sweep value, its fixed full-panel plan, and the
//! unforgeable stripe/sweep witnesses.
//!
//! Exit-review restructuring through round 3 (round-1 findings 4, 5, 9;
//! round-3 findings 1–3):
//!
//! - coverage consumes **every transfer settlement**, not caller claims:
//!   [`crate::transfer::Settled::into_parts`] returns exactly one
//!   [`StripeSettlement`]; written settlements advance, while failed or
//!   cancelled settlements poison the epoch and leave only abort;
//! - the sweep value is crate-owned and binds the demand-fixed panel plan,
//!   the scene snapshot (owned, exposed by shared reference only), the
//!   repaint mode, and the provenance-branded epoch — there is no public
//!   path to attach a foreign or trivial plan. Interior mutability and
//!   shared external state remain documented snapshot escape surfaces;
//! - milestone vocabulary is honest: the terminal witness is
//!   [`SweepWritten`] — every planned stripe was *written*; nothing here
//!   claims physical presentation.

use crate::geometry::{FrameEpoch, PanelGeometry, Region};
use crate::transfer::TransferOutcome;

/// An unforgeable stripe target: the identity (demand, epoch, region) a
/// transfer must carry to witness coverage. Minted only by
/// [`Sweep::next_target`]; non-`Clone`, private fields, and consumed by
/// [`StripeTarget::start_flight`]. The target itself supplies the starter's
/// region, so public code cannot independently pass a claimed identity into
/// an in-flight carrier (exit-review round-3 finding 1).
#[derive(Debug)]
pub struct StripeTarget {
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
    pub(crate) region: Region,
}

impl StripeTarget {
    /// The region this target covers.
    pub const fn region(&self) -> Region {
        self.region
    }

    /// The epoch this target belongs to.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }
}

/// Witness that one stripe's transfer settled `Completed`. Minted exactly
/// once in the [`StripeSettlement::Written`] arm returned by
/// [`crate::transfer::Settled::into_parts`]; non-`Clone`, private fields.
#[must_use = "an unreconciled written witness is lost coverage"]
#[derive(Debug)]
pub struct StripeWritten {
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
    pub(crate) region: Region,
}

impl StripeWritten {
    /// The epoch the written stripe belongs to.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }

    /// The written region.
    pub const fn region(&self) -> Region {
        self.region
    }
}

/// Witness that one stripe settled without a complete write. It carries the
/// real `Cancelled` or `Failed` recovery outcome plus private target identity;
/// safe code can neither forge one nor relabel it as [`StripeWritten`].
#[must_use = "an unreconciled unwritten stripe leaves its sweep outstanding"]
#[derive(Debug)]
pub struct StripeUnwritten {
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
    pub(crate) region: Region,
    pub(crate) outcome: TransferOutcome,
}

impl StripeUnwritten {
    /// The epoch whose sweep must be poisoned by this settlement.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }

    /// The target region that did not settle written.
    pub const fn region(&self) -> Region {
        self.region
    }

    /// The integration-reported reason: always `Cancelled` or `Failed` for
    /// values minted by the private settlement path.
    pub const fn outcome(&self) -> TransferOutcome {
        self.outcome
    }
}

/// The mandatory move-only reconciliation witness for one started transfer.
/// Separate unforgeable inner types prevent safe code from rewriting a failed
/// or cancelled recovery into written coverage.
#[must_use = "every transfer settlement must be reconciled with its sweep"]
#[derive(Debug)]
pub enum StripeSettlement {
    /// A real completed write; accepting it advances coverage once.
    Written(StripeWritten),
    /// A failed or cancelled write; accepting it poisons the sweep.
    Unwritten(StripeUnwritten),
}

impl StripeSettlement {
    /// The transfer recovery outcome carried by this witness.
    pub const fn outcome(&self) -> TransferOutcome {
        match self {
            Self::Written(_) => TransferOutcome::Completed,
            Self::Unwritten(unwritten) => unwritten.outcome,
        }
    }

    /// The epoch this settlement must reconcile with.
    pub const fn epoch(&self) -> FrameEpoch {
        match self {
            Self::Written(written) => written.epoch,
            Self::Unwritten(unwritten) => unwritten.epoch,
        }
    }

    /// The exact plan region this settlement targets.
    pub const fn region(&self) -> Region {
        match self {
            Self::Written(written) => written.region,
            Self::Unwritten(unwritten) => unwritten.region,
        }
    }

    const fn demand_id(&self) -> u64 {
        match self {
            Self::Written(written) => written.demand_id,
            Self::Unwritten(unwritten) => unwritten.demand_id,
        }
    }
}

/// A validated full-panel stripe plan. Fixed at
/// [`crate::demand::FrameDemand`] construction; sweeps cannot substitute
/// another.
#[derive(Clone, Copy, Debug)]
pub struct SweepPlan {
    panel: Region,
    stripe_height: u16,
}

/// The plan could not be validated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvalidPlan {
    /// Panel width or height is zero.
    EmptyPanel,
    /// Stripe height is zero.
    ZeroStripe,
    /// Panel extents overflow the coordinate space.
    Overflow,
}

impl SweepPlan {
    /// Validates a full-panel plan over an **admitted geometry** (round-2
    /// finding 5): the panel comes from [`PanelGeometry`], whose custom
    /// constructor is the named escape. The final stripe may be shorter
    /// than `stripe_height`; coverage is exact by construction.
    ///
    /// # Errors
    ///
    /// [`InvalidPlan`] for an empty panel, a zero stripe height, or panel
    /// extents that overflow `u16` coordinates.
    pub const fn for_panel(
        geometry: PanelGeometry,
        stripe_height: u16,
    ) -> Result<Self, InvalidPlan> {
        Self::new(geometry.panel(), stripe_height)
    }

    pub(crate) const fn new(panel: Region, stripe_height: u16) -> Result<Self, InvalidPlan> {
        if panel.width == 0 || panel.height == 0 {
            return Err(InvalidPlan::EmptyPanel);
        }
        if stripe_height == 0 {
            return Err(InvalidPlan::ZeroStripe);
        }
        if panel.x.checked_add(panel.width).is_none() || panel.y.checked_add(panel.height).is_none()
        {
            return Err(InvalidPlan::Overflow);
        }
        Ok(Self {
            panel,
            stripe_height,
        })
    }

    /// Number of stripes in the plan.
    pub const fn stripe_count(&self) -> u16 {
        self.panel.height.div_ceil(self.stripe_height)
    }

    /// The `index`-th stripe region, in global panel coordinates.
    pub const fn region_at(&self, index: u16) -> Option<Region> {
        if index >= self.stripe_count() {
            return None;
        }
        let offset = index * self.stripe_height;
        let remaining = self.panel.height - offset;
        let height = if remaining < self.stripe_height {
            remaining
        } else {
            self.stripe_height
        };
        Some(Region {
            x: self.panel.x,
            y: self.panel.y + offset,
            width: self.panel.width,
            height,
        })
    }
}

/// Witness that every planned stripe of the sweep was written, in order.
/// The only success value [`crate::demand::FrameDemand::finish_written`]
/// accepts. Carries its demand provenance; a foreign demand rejects it
/// without mutating (finding 6).
#[must_use = "an unsettled sweep witness wedges the demand machine"]
#[derive(Debug)]
pub struct SweepWritten {
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
}

/// Witness of an aborted sweep — transfer failure, cancellation, or
/// shutdown. The only value [`crate::demand::FrameDemand::finish_failed`]
/// accepts.
#[must_use = "an unsettled abort witness wedges the demand machine"]
#[derive(Debug)]
pub struct AbortedSweep {
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
}

/// A settlement did not match the sweep's one outstanding target. This also
/// covers foreign provenance, settlement with no target outstanding, and any
/// attempt after poison; rejection leaves all sweep state unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WrongStripe {
    /// The region the plan expected next, if any remained.
    pub expected: Option<Region>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SweepState {
    Ready,
    Outstanding,
    Poisoned,
}

/// One crate-owned sweep: the scene snapshot, the demand-fixed
/// plan, the repaint mode, and the branded epoch. Minted only by
/// [`crate::demand::FrameDemand::begin_sweep`].
///
/// The snapshot is owned here and exposed by shared reference only (SPEC
/// 6.3). Ordinary Rust borrowing prevents mutation through a plain `&S`,
/// but `S` is unconstrained: interior mutability or shared external handles
/// can still change the rendered state. Keeping those stable for the epoch
/// is a documented caller obligation, not a type-level guarantee.
pub struct Sweep<S> {
    snapshot: S,
    plan: SweepPlan,
    next: u16,
    state: SweepState,
    full_repaint: bool,
    pub(crate) demand_id: u64,
    pub(crate) epoch: FrameEpoch,
}

impl<S> core::fmt::Debug for Sweep<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sweep")
            .field("epoch", &self.epoch)
            .field("next", &self.next)
            .field("state", &self.state)
            .field("full_repaint", &self.full_repaint)
            .finish_non_exhaustive()
    }
}

impl<S> Sweep<S> {
    pub(crate) const fn mint(
        snapshot: S,
        plan: SweepPlan,
        full_repaint: bool,
        demand_id: u64,
        epoch: FrameEpoch,
    ) -> Self {
        Self {
            snapshot,
            plan,
            next: 0,
            state: SweepState::Ready,
            full_repaint,
            demand_id,
            epoch,
        }
    }

    /// The scene snapshot for this epoch, exposed only by shared reference.
    /// Interior mutability and shared external state remain caller-owned
    /// escape surfaces.
    pub const fn snapshot(&self) -> &S {
        &self.snapshot
    }

    /// Whether a recovery or invalidation obligation requires this sweep.
    /// K2R-0 plans cover the full panel regardless; `false` means there is
    /// no outstanding forced-repaint obligation, not that the sweep is
    /// partial.
    pub const fn full_repaint(&self) -> bool {
        self.full_repaint
    }

    /// The epoch under sweep.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }

    /// The current plan position, if any remains. This is introspection only:
    /// it stays `Some` while that position has a target outstanding and after
    /// poison; [`Sweep::next_target`] is the issuance authority.
    pub const fn next_region(&self) -> Option<Region> {
        self.plan.region_at(self.next)
    }

    /// Mints the one unforgeable target for the current planned stripe.
    /// While it is outstanding, or after any failed/cancelled settlement has
    /// poisoned the sweep, another mint returns `None` without mutation.
    pub fn next_target(&mut self) -> Option<StripeTarget> {
        if self.state != SweepState::Ready {
            return None;
        }
        let region = self.plan.region_at(self.next)?;
        self.state = SweepState::Outstanding;
        Some(StripeTarget {
            demand_id: self.demand_id,
            epoch: self.epoch,
            region,
        })
    }

    /// Reconciles the one outstanding transfer. Matching written settlement
    /// clears outstanding and advances coverage once; matching failed or
    /// cancelled settlement clears outstanding and irreversibly poisons this
    /// epoch, leaving [`Sweep::abort`] as its only terminal transition.
    ///
    /// # Errors
    ///
    /// [`WrongStripe`] for foreign demand/epoch/region identity, no target
    /// outstanding, a fully covered sweep, or an already-poisoned sweep. All
    /// observable sweep state is unchanged.
    #[allow(clippy::needless_pass_by_value)] // witness consumption is the contract
    pub fn settle(&mut self, settlement: StripeSettlement) -> Result<TransferOutcome, WrongStripe> {
        let expected = self.next_region();
        if self.state != SweepState::Outstanding
            || settlement.demand_id() != self.demand_id
            || settlement.epoch() != self.epoch
            || Some(settlement.region()) != expected
        {
            return Err(WrongStripe { expected });
        }

        let outcome = settlement.outcome();
        match settlement {
            StripeSettlement::Written(_) => {
                self.next += 1;
                self.state = SweepState::Ready;
            }
            StripeSettlement::Unwritten(_) => {
                self.state = SweepState::Poisoned;
            }
        }
        Ok(outcome)
    }

    /// Whether every planned stripe has been written and no target remains
    /// outstanding. A poisoned sweep is never complete.
    pub const fn is_complete(&self) -> bool {
        matches!(self.state, SweepState::Ready) && self.next >= self.plan.stripe_count()
    }

    /// Whether a failed or cancelled settlement made successful completion
    /// impossible. Poison is irreversible; only [`Sweep::abort`] remains.
    pub const fn is_poisoned(&self) -> bool {
        matches!(self.state, SweepState::Poisoned)
    }

    /// Consumes a fully covered sweep into its terminal witness, returning
    /// the snapshot to the caller.
    ///
    /// # Errors
    ///
    /// Returns the sweep unchanged while a target is outstanding, stripes
    /// remain unwritten, or the sweep is poisoned.
    pub fn finish(self) -> Result<(SweepWritten, S), Sweep<S>> {
        if self.is_complete() {
            Ok((
                SweepWritten {
                    demand_id: self.demand_id,
                    epoch: self.epoch,
                },
                self.snapshot,
            ))
        } else {
            Err(self)
        }
    }

    /// Aborts the sweep at any point, including shutdown with an outstanding
    /// target or flight, and returns the snapshot. Bookkeeping cannot revoke
    /// either value: a retained target can still be started and a live flight
    /// can still write. Drop the target and drain the flight before replacement
    /// whenever shutdown permits it. Accepting the abort forces a full repaint;
    /// if a stale write can overlap a replacement, call
    /// [`crate::demand::FrameDemand::invalidate`] so that replacement is
    /// discarded and another full repaint remains due.
    pub fn abort(self) -> (AbortedSweep, S) {
        (
            AbortedSweep {
                demand_id: self.demand_id,
                epoch: self.epoch,
            },
            self.snapshot,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: Region = Region {
        x: 0,
        y: 0,
        width: 8,
        height: 4,
    };

    fn plan() -> SweepPlan {
        SweepPlan::new(PANEL, 2).expect("valid test plan")
    }

    fn written(demand_id: u64, epoch: FrameEpoch, region: Region) -> StripeSettlement {
        StripeSettlement::Written(StripeWritten {
            demand_id,
            epoch,
            region,
        })
    }

    fn unwritten(demand_id: u64, epoch: FrameEpoch, region: Region) -> StripeSettlement {
        StripeSettlement::Unwritten(StripeUnwritten {
            demand_id,
            epoch,
            region,
            outcome: TransferOutcome::Failed,
        })
    }

    fn private_state(sweep: &Sweep<()>) -> (Region, u16, u16, SweepState, bool, u64, FrameEpoch) {
        (
            sweep.plan.panel,
            sweep.plan.stripe_height,
            sweep.next,
            sweep.state,
            sweep.full_repaint,
            sweep.demand_id,
            sweep.epoch,
        )
    }

    fn assert_rejected_unchanged(sweep: &mut Sweep<()>, settlement: StripeSettlement) {
        let before = private_state(sweep);
        let expected = sweep.next_region();
        assert_eq!(sweep.settle(settlement), Err(WrongStripe { expected }));
        assert_eq!(private_state(sweep), before);
    }

    #[test]
    fn settle_rejection_predicates_preserve_all_private_state() {
        const DEMAND_ID: u64 = 7;
        const EPOCH: FrameEpoch = FrameEpoch(3);

        // These deliberately crate-private constructions exercise defensive
        // branches that safe external code cannot isolate; the UI suite proves
        // the witnesses themselves remain unforgeable outside the crate.
        let mut sweep = Sweep::mint((), plan(), true, DEMAND_ID, EPOCH);
        let first = sweep.next_region().expect("first region");
        assert_rejected_unchanged(&mut sweep, written(DEMAND_ID, EPOCH, first)); // no target outstanding

        let issued = sweep.next_target().expect("issue first target");
        assert_eq!(issued.region(), first);
        assert_rejected_unchanged(
            &mut sweep,
            written(DEMAND_ID, FrameEpoch(EPOCH.0 - 1), first),
        ); // stale epoch
        assert_rejected_unchanged(&mut sweep, written(DEMAND_ID + 1, EPOCH, first)); // foreign demand
        let second = plan().region_at(1).expect("second region");
        assert_rejected_unchanged(&mut sweep, written(DEMAND_ID, EPOCH, second)); // wrong region

        assert_eq!(
            sweep.settle(written(DEMAND_ID, EPOCH, first)),
            Ok(TransferOutcome::Completed)
        );
        let issued = sweep.next_target().expect("issue second target");
        assert_eq!(issued.region(), second);
        assert_eq!(
            sweep.settle(written(DEMAND_ID, EPOCH, second)),
            Ok(TransferOutcome::Completed)
        );
        assert!(sweep.is_complete());
        assert_rejected_unchanged(&mut sweep, written(DEMAND_ID, EPOCH, second)); // fully covered

        let mut poisoned = Sweep::mint((), plan(), true, DEMAND_ID, EPOCH);
        let issued = poisoned.next_target().expect("issue poison target");
        assert_eq!(issued.region(), first);
        assert_eq!(
            poisoned.settle(unwritten(DEMAND_ID, EPOCH, first)),
            Ok(TransferOutcome::Failed)
        );
        assert!(poisoned.is_poisoned());
        assert_rejected_unchanged(&mut poisoned, written(DEMAND_ID, EPOCH, first));
    }
}
