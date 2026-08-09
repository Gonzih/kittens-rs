//! Frame-demand policy: coalescing requests, one sweep in flight,
//! throttle eligibility, and the full-repaint obligation — driven by
//! unforgeable sweep tokens (review finding 10).
//!
//! Time is the crate-owned monotonic [`Tick`] (review finding 16): the
//! platform defines the unit; host tests use plain numbers; the target maps
//! its timer. There is no `on_eligible` — [`FrameDemand::begin_sweep`] is
//! the sole operation that acknowledges elapsed eligibility (finding 18).

use crate::geometry::FrameEpoch;
use crate::sweep::{AbortedSweep, CompletedSweep, SweepToken};

/// A crate-owned monotonic instant in platform-defined tick units.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tick(pub u64);

/// The demand policy state machine.
///
/// State table (normative rows exercised one-to-one by the oracle suite):
///
/// | State | Operation | Result |
/// |---|---|---|
/// | clean, idle | `request` | dirty |
/// | clean, idle | `begin_sweep` | `None` |
/// | dirty, idle, throttle elapsed | `begin_sweep` | mints token, epoch+1, dirty cleared, schedule cleared |
/// | dirty, idle, throttled | `begin_sweep` | `None`, `eligible_at` scheduled |
/// | any, sweeping | `begin_sweep` | `None` (one sweep in flight) |
/// | sweeping | `request` | dirty (targets the *next* epoch, survives settlement) |
/// | sweeping | `finish_presented(completed, now)` | idle, throttle advances to `now`; clears full-repaint only if no invalidation since mint |
/// | sweeping | `finish_failed(aborted, now)` | idle, dirty retained, full-repaint set, throttle NOT advanced |
/// | any | `invalidate` | dirty, full-repaint set, invalidation generation bumped |
#[derive(Debug)]
pub struct FrameDemand {
    dirty: bool,
    sweeping: Option<FrameEpoch>,
    last_present: Option<Tick>,
    scheduled: Option<Tick>,
    min_interval: u64,
    next_epoch: u64,
    full_repaint: bool,
    invalidations: u32,
}

impl FrameDemand {
    /// Explicit throttle policy in tick units; no `Default`.
    pub const fn new(min_interval_ticks: u64) -> Self {
        Self {
            dirty: false,
            sweeping: None,
            last_present: None,
            scheduled: None,
            min_interval: min_interval_ticks,
            next_epoch: 0,
            full_repaint: true, // first-ever sweep paints everything
            invalidations: 0,
        }
    }

    /// Coalescing demand — the only vocabulary shared with kittens-tui.
    pub fn request(&mut self) {
        self.dirty = true;
    }

    /// Whether demand is pending.
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The epoch currently under sweep, if any.
    pub const fn sweeping(&self) -> Option<FrameEpoch> {
        self.sweeping
    }

    /// Whether the next sweep must repaint the full panel. Initially true;
    /// set by failures and invalidations; cleared by a presented sweep
    /// minted after the last invalidation.
    pub const fn full_repaint_required(&self) -> bool {
        self.full_repaint
    }

    /// External invalidation: transport reset, panel reinitialization, or
    /// any epoch discontinuity. Demand is raised and the full-repaint
    /// obligation is recorded; a sweep already in flight can no longer
    /// clear that obligation (its token predates this invalidation).
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.full_repaint = true;
        self.invalidations = self.invalidations.wrapping_add(1);
    }

    /// Mints the sweep token when a sweep is due: demand pending, no sweep
    /// in flight, throttle elapsed. The throttled case records the instant
    /// exposed by [`FrameDemand::eligible_at`]. This is also the sole
    /// acknowledgment of elapsed eligibility.
    pub fn begin_sweep(&mut self, now: Tick) -> Option<SweepToken> {
        if !self.dirty || self.sweeping.is_some() {
            return None;
        }
        if let Some(last) = self.last_present {
            let eligible = Tick(last.0.saturating_add(self.min_interval));
            if now < eligible {
                self.scheduled = Some(eligible);
                return None;
            }
        }
        let epoch = FrameEpoch(self.next_epoch);
        self.next_epoch += 1;
        self.dirty = false;
        self.scheduled = None;
        self.sweeping = Some(epoch);
        Some(SweepToken {
            epoch,
            invalidations_at_mint: self.invalidations,
        })
    }

    /// Earliest eligible sweep instant; `Some` only while demand is
    /// throttle-blocked and no sweep is in flight. Feed a deadline source
    /// from this in `before_poll`.
    pub const fn eligible_at(&self) -> Option<Tick> {
        if self.dirty && self.sweeping.is_none() {
            self.scheduled
        } else {
            None
        }
    }

    /// Settles the active sweep as presented: every planned region was
    /// written, witnessed by [`CompletedSweep`]. Advances the throttle to
    /// `now`. Clears the full-repaint obligation only when no invalidation
    /// occurred since the token was minted.
    pub fn finish_presented(&mut self, completed: CompletedSweep, now: Tick) {
        let token = completed.token;
        debug_assert_eq!(
            Some(token.epoch),
            self.sweeping,
            "token matches active sweep"
        );
        self.sweeping = None;
        self.last_present = Some(now);
        self.scheduled = None;
        if token.invalidations_at_mint == self.invalidations {
            self.full_repaint = false;
        }
    }

    /// Settles the active sweep as failed/aborted. Demand is retained (the
    /// frame was wanted and was not shown), the full-repaint obligation is
    /// recorded, and the throttle does **not** advance — retry pacing under
    /// persistent failure is application policy, not silent suppression.
    pub fn finish_failed(&mut self, aborted: AbortedSweep, _now: Tick) {
        let token = aborted.token;
        debug_assert_eq!(
            Some(token.epoch),
            self.sweeping,
            "token matches active sweep"
        );
        self.sweeping = None;
        self.dirty = true;
        self.full_repaint = true;
    }
}
