//! Frame-demand policy: coalescing requests, one sweep in flight, throttle
//! eligibility, invalidation, and recovery — with provenance-branded
//! settlement (exit-review round 1, findings 6–9).
//!
//! Vocabulary honesty: the throttle milestone is *written*, never
//! "presented" — transfer completion says nothing about physical
//! presentation (finding 9).

use core::sync::atomic::{AtomicU32, Ordering};

use crate::geometry::FrameEpoch;
use crate::sweep::{AbortedSweep, Sweep, SweepPlan, SweepWritten};

/// A crate-owned monotonic instant in platform-defined tick units.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tick(pub u64);

/// A settlement witness carried provenance from a different `FrameDemand`
/// instance or a non-active sweep. Nothing was mutated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForeignSweep;

/// How a written sweep settled against the demand machine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WrittenDisposition {
    /// The sweep counts: throttle advanced, obligations cleared.
    Effective,
    /// The panel was invalidated while this sweep was in flight; its output
    /// is suspect. Demand and the full-repaint obligation are retained and
    /// the throttle did not advance (finding 7).
    DiscardedByInvalidation,
}

static DEMAND_IDS: AtomicU32 = AtomicU32::new(0);

/// The demand policy state machine. Owns the fixed, validated panel plan;
/// sweeps cannot substitute another (finding 5).
///
/// State table (each row has an oracle):
///
/// | State | Operation | Result |
/// |---|---|---|
/// | clean, idle | `request` | dirty |
/// | clean, idle | `begin_sweep` | `None` |
/// | dirty, idle, throttle elapsed | `begin_sweep` | mints `Sweep`, epoch+1, dirty cleared |
/// | dirty, idle, throttled | `begin_sweep` | `None`, `eligible_at` scheduled |
/// | any, sweeping | `begin_sweep` | `None` (one sweep in flight) |
/// | sweeping | `request` | dirty (targets the next epoch, survives settlement) |
/// | sweeping | `finish_written` (active token, not invalidated) | idle, throttle → `now`, obligations cleared: `Effective` |
/// | sweeping | `finish_written` (active token, invalidated mid-sweep) | idle, dirty + full-repaint retained, throttle unchanged: `DiscardedByInvalidation` |
/// | any | `finish_written`/`finish_failed` (foreign/stale token) | `Err(ForeignSweep)`, **no mutation** |
/// | sweeping | `finish_failed` (active token) | idle, dirty retained, full-repaint set, throttle unchanged |
/// | sweeping | `abandon_active` | idle, dirty + full-repaint set, throttle unchanged (dropped-sweep recovery, finding 8) |
/// | any | `invalidate` | dirty + full-repaint set; an active sweep is marked non-clearing |
#[derive(Debug)]
pub struct FrameDemand {
    id: u32,
    plan: SweepPlan,
    dirty: bool,
    sweeping: Option<FrameEpoch>,
    active_invalidated: bool,
    last_written: Option<Tick>,
    scheduled: Option<Tick>,
    min_interval: u64,
    next_epoch: u64,
    full_repaint: bool,
}

impl FrameDemand {
    /// Explicit throttle policy (tick units) and the fixed panel plan; no
    /// `Default`.
    pub fn new(min_interval_ticks: u64, plan: SweepPlan) -> Self {
        Self {
            id: DEMAND_IDS.fetch_add(1, Ordering::Relaxed),
            plan,
            dirty: false,
            sweeping: None,
            active_invalidated: false,
            last_written: None,
            scheduled: None,
            min_interval: min_interval_ticks,
            next_epoch: 0,
            full_repaint: true, // the first-ever sweep paints everything
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

    /// Whether the next sweep must repaint the full panel.
    pub const fn full_repaint_required(&self) -> bool {
        self.full_repaint
    }

    /// External invalidation: transport reset, panel reinitialization, any
    /// epoch discontinuity. Raises demand, records the full-repaint
    /// obligation, and marks an in-flight sweep non-clearing: its eventual
    /// written settlement is discarded rather than trusted (finding 7). A
    /// bool latch, not a wrappable counter.
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.full_repaint = true;
        if self.sweeping.is_some() {
            self.active_invalidated = true;
        }
    }

    /// Mints the sweep when one is due: demand pending, no sweep in
    /// flight, throttle elapsed. Binds the immutable snapshot, the fixed
    /// plan, the repaint mode, and the branded epoch. The throttled case
    /// records [`FrameDemand::eligible_at`]; calling this is also the sole
    /// acknowledgment of elapsed eligibility.
    pub fn begin_sweep<S>(&mut self, now: Tick, snapshot: S) -> Option<Sweep<S>> {
        if !self.dirty || self.sweeping.is_some() {
            return None;
        }
        if let Some(last) = self.last_written {
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
        self.active_invalidated = false;
        Some(Sweep::mint(
            snapshot,
            self.plan,
            self.full_repaint,
            self.id,
            epoch,
        ))
    }

    /// Earliest eligible sweep instant; `Some` only while demand is
    /// throttle-blocked and no sweep is in flight.
    pub const fn eligible_at(&self) -> Option<Tick> {
        if self.dirty && self.sweeping.is_none() {
            self.scheduled
        } else {
            None
        }
    }

    /// Settles the active sweep as written (full coverage witnessed by
    /// [`SweepWritten`]).
    ///
    /// # Errors
    ///
    /// [`ForeignSweep`] — witness from another demand instance or a
    /// non-active epoch; **nothing is mutated** (finding 6, checked in
    /// release builds, not `debug_assert`ed).
    // Consuming the witness by value is the contract: a settled witness is
    // spent and cannot be replayed against another demand instance.
    #[allow(clippy::needless_pass_by_value)]
    pub fn finish_written(
        &mut self,
        written: SweepWritten,
        now: Tick,
    ) -> Result<WrittenDisposition, ForeignSweep> {
        if written.demand_id != self.id || Some(written.epoch) != self.sweeping {
            return Err(ForeignSweep);
        }
        self.sweeping = None;
        if self.active_invalidated {
            // The panel changed under this sweep: output is suspect. Retain
            // demand and the obligation; do not advance the throttle.
            self.active_invalidated = false;
            self.dirty = true;
            self.full_repaint = true;
            return Ok(WrittenDisposition::DiscardedByInvalidation);
        }
        self.last_written = Some(now);
        self.scheduled = None;
        self.full_repaint = false;
        Ok(WrittenDisposition::Effective)
    }

    /// Settles the active sweep as failed/aborted: demand retained, the
    /// full-repaint obligation recorded, the throttle not advanced.
    ///
    /// # Errors
    ///
    /// [`ForeignSweep`] — witness from another demand instance or a
    /// non-active epoch; nothing is mutated.
    #[allow(clippy::needless_pass_by_value)] // witness consumption is the contract
    pub fn finish_failed(&mut self, aborted: AbortedSweep, _now: Tick) -> Result<(), ForeignSweep> {
        if aborted.demand_id != self.id || Some(aborted.epoch) != self.sweeping {
            return Err(ForeignSweep);
        }
        self.sweeping = None;
        self.active_invalidated = false;
        self.dirty = true;
        self.full_repaint = true;
        Ok(())
    }

    /// Conservative recovery when the active sweep's value was dropped
    /// (early return, panic path, lost token): clears the active epoch,
    /// retains demand, forces a full repaint, does not advance the
    /// throttle. Idempotent when idle (finding 8).
    pub fn abandon_active(&mut self) {
        if self.sweeping.take().is_some() {
            self.active_invalidated = false;
            self.dirty = true;
            self.full_repaint = true;
        }
    }
}
