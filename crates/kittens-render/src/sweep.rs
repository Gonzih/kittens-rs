//! The fixed full-panel sweep plan and its consuming progress token.
//!
//! Review finding 9 established that sweep completion must never be a
//! caller assertion: two overlapping writes that miss a row could otherwise
//! be declared "every stripe". Here, the only way to obtain a
//! [`CompletedSweep`] — the only value [`crate::demand::FrameDemand`]
//! accepts as a presented sweep — is to mark every planned region written,
//! in order. Coverage is a construction, not a claim.
//!
//! K2R-0 deliberately supports exactly one plan shape: the full panel in
//! top-to-bottom stripes (damage sweeps are deferred; SPEC section 4).

use crate::geometry::{FrameEpoch, Region};

/// An unforgeable witness of one active sweep. Minted only by
/// [`crate::demand::FrameDemand::begin_sweep`]; consumed by settlement.
/// Non-`Clone`, no public constructor: at most one exists per sweep.
#[derive(Debug)]
pub struct SweepToken {
    pub(crate) epoch: FrameEpoch,
    pub(crate) invalidations_at_mint: u32,
}

impl SweepToken {
    /// The epoch this sweep renders.
    pub const fn epoch(&self) -> FrameEpoch {
        self.epoch
    }
}

/// A validated full-panel stripe plan.
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
}

impl SweepPlan {
    /// Validates a full-panel plan. The final stripe may be shorter than
    /// `stripe_height`; coverage is exact by construction.
    ///
    /// # Errors
    ///
    /// [`InvalidPlan`] for an empty panel or a zero stripe height.
    pub const fn new(panel: Region, stripe_height: u16) -> Result<Self, InvalidPlan> {
        if panel.width == 0 || panel.height == 0 {
            return Err(InvalidPlan::EmptyPanel);
        }
        if stripe_height == 0 {
            return Err(InvalidPlan::ZeroStripe);
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

/// Progress through one sweep: regions must be marked written in plan
/// order; completion is obtainable only after every region is written.
#[derive(Debug)]
pub struct SweepProgress {
    plan: SweepPlan,
    token: SweepToken,
    next: u16,
}

/// The marked region was not the next planned region.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WrongRegion {
    /// The region the plan expected next, if any.
    pub expected: Option<Region>,
}

/// Witness that every planned region of the sweep was written, in order.
/// The only value accepted by `FrameDemand::finish_presented`.
#[derive(Debug)]
pub struct CompletedSweep {
    pub(crate) token: SweepToken,
}

/// Witness of an aborted sweep. The only value accepted by
/// `FrameDemand::finish_failed`.
#[derive(Debug)]
pub struct AbortedSweep {
    pub(crate) token: SweepToken,
}

impl SweepProgress {
    /// Binds a sweep token to the plan it renders.
    pub const fn new(plan: SweepPlan, token: SweepToken) -> Self {
        Self {
            plan,
            token,
            next: 0,
        }
    }

    /// The epoch under sweep.
    pub const fn epoch(&self) -> FrameEpoch {
        self.token.epoch
    }

    /// The next region to render and write, if any remain.
    pub const fn next_region(&self) -> Option<Region> {
        self.plan.region_at(self.next)
    }

    /// Records that `region` was written (its transfer settled
    /// `Completed`). Must be exactly [`SweepProgress::next_region`].
    ///
    /// # Errors
    ///
    /// [`WrongRegion`] when `region` is out of order or the sweep is
    /// already fully covered; progress is unchanged.
    pub fn mark_written(&mut self, region: Region) -> Result<(), WrongRegion> {
        match self.next_region() {
            Some(expected) if expected == region => {
                self.next += 1;
                Ok(())
            }
            expected => Err(WrongRegion { expected }),
        }
    }

    /// Whether every planned region has been written.
    pub const fn is_complete(&self) -> bool {
        self.next >= self.plan.stripe_count()
    }

    /// Consumes fully covered progress into the presented-sweep witness.
    ///
    /// # Errors
    ///
    /// Returns the progress unchanged while regions remain unwritten.
    pub fn complete(self) -> Result<CompletedSweep, SweepProgress> {
        if self.is_complete() {
            Ok(CompletedSweep { token: self.token })
        } else {
            Err(self)
        }
    }

    /// Aborts the sweep at any point (transfer failure, cancellation,
    /// shutdown). The epoch terminates; `FrameDemand::finish_failed`
    /// retains demand and records the full-repaint obligation.
    pub fn abort(self) -> AbortedSweep {
        AbortedSweep { token: self.token }
    }
}
