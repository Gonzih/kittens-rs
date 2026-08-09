//! The crate-owned sweep value, its fixed full-panel plan, and the
//! unforgeable stripe/sweep witnesses.
//!
//! Exit-review round 1 restructuring (findings 4, 5, 9):
//!
//! - coverage consumes **transfer outcomes**, not caller claims: the only
//!   way to mark a stripe is a [`StripeWritten`] witness, and the only mint
//!   for that witness is a settled transfer whose outcome was `Completed`
//!   ([`crate::transfer::Settled::stripe_written`]);
//! - the sweep value is crate-owned and binds the demand-fixed panel plan,
//!   the immutable scene snapshot (owned, exposed by shared reference
//!   only), the repaint mode, and the provenance-branded epoch — there is
//!   no public path to attach a foreign or trivial plan;
//! - milestone vocabulary is honest: the terminal witness is
//!   [`SweepWritten`] — every planned stripe was *written*; nothing here
//!   claims physical presentation.

use crate::geometry::{FrameEpoch, Region};

/// Witness that one stripe's transfer settled `Completed`. Minted only by
/// [`crate::transfer::Settled::stripe_written`]; non-`Clone`, no public
/// constructor. A cancelled, failed, or never-started stripe has no witness
/// and therefore cannot be marked.
#[derive(Debug)]
pub struct StripeWritten {
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
    /// Validates a full-panel plan. The final stripe may be shorter than
    /// `stripe_height`; coverage is exact by construction.
    ///
    /// # Errors
    ///
    /// [`InvalidPlan`] for an empty panel, a zero stripe height, or panel
    /// extents that overflow `u16` coordinates.
    pub const fn new(panel: Region, stripe_height: u16) -> Result<Self, InvalidPlan> {
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
#[derive(Debug)]
pub struct SweepWritten {
    pub(crate) demand_id: u32,
    pub(crate) epoch: FrameEpoch,
}

/// Witness of an aborted sweep — transfer failure, cancellation, or
/// shutdown. The only value [`crate::demand::FrameDemand::finish_failed`]
/// accepts.
#[derive(Debug)]
pub struct AbortedSweep {
    pub(crate) demand_id: u32,
    pub(crate) epoch: FrameEpoch,
}

/// The marked witness did not match the sweep's next planned stripe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WrongStripe {
    /// The region the plan expected next, if any remained.
    pub expected: Option<Region>,
}

/// One crate-owned sweep: the immutable scene snapshot, the demand-fixed
/// plan, the repaint mode, and the branded epoch. Minted only by
/// [`crate::demand::FrameDemand::begin_sweep`].
///
/// The snapshot is owned here and exposed by shared reference only — the
/// scene cannot be mutated through the sweep, so every stripe of the epoch
/// renders one state (SPEC 6.4 rule 1's enforcement at this layer).
pub struct Sweep<S> {
    snapshot: S,
    plan: SweepPlan,
    next: u16,
    full_repaint: bool,
    pub(crate) demand_id: u32,
    pub(crate) epoch: FrameEpoch,
}

impl<S> core::fmt::Debug for Sweep<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sweep")
            .field("epoch", &self.epoch)
            .field("next", &self.next)
            .field("full_repaint", &self.full_repaint)
            .finish_non_exhaustive()
    }
}

impl<S> Sweep<S> {
    pub(crate) const fn mint(
        snapshot: S,
        plan: SweepPlan,
        full_repaint: bool,
        demand_id: u32,
        epoch: FrameEpoch,
    ) -> Self {
        Self {
            snapshot,
            plan,
            next: 0,
            full_repaint,
            demand_id,
            epoch,
        }
    }

    /// The immutable scene snapshot for this epoch.
    pub const fn snapshot(&self) -> &S {
        &self.snapshot
    }

    /// Whether this sweep must repaint everything (always true for the
    /// K2R-0 full-panel plan; carried for damage-sweep forward shape).
    pub const fn full_repaint(&self) -> bool {
        self.full_repaint
    }

    /// The epoch under sweep.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }

    /// The next region to render and transfer, if any remain.
    pub const fn next_region(&self) -> Option<Region> {
        self.plan.region_at(self.next)
    }

    /// Records one written stripe by consuming its transfer witness. The
    /// witness must carry this sweep's epoch and exactly the next planned
    /// region.
    ///
    /// # Errors
    ///
    /// [`WrongStripe`] for an out-of-order region, a foreign epoch, or a
    /// fully covered sweep; progress is unchanged.
    pub fn mark_written(&mut self, witness: StripeWritten) -> Result<(), WrongStripe> {
        match self.next_region() {
            Some(expected) if witness.epoch == self.epoch && witness.region == expected => {
                self.next += 1;
                Ok(())
            }
            expected => Err(WrongStripe { expected }),
        }
    }

    /// Whether every planned stripe has been written.
    pub const fn is_complete(&self) -> bool {
        self.next >= self.plan.stripe_count()
    }

    /// Consumes a fully covered sweep into its terminal witness, returning
    /// the snapshot to the caller.
    ///
    /// # Errors
    ///
    /// Returns the sweep unchanged while stripes remain unwritten.
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

    /// Aborts the sweep at any point, returning the snapshot. The epoch
    /// terminates; `finish_failed` retains demand and records the
    /// full-repaint obligation.
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
