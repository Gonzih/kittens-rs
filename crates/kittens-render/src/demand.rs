//! Frame-demand policy: coalescing requests, one machine-active sweep epoch,
//! throttle eligibility, invalidation, and recovery — with
//! provenance-branded settlement (exit-review round 1, findings 6–9).
//!
//! Vocabulary honesty: the throttle milestone is *written*, never
//! "presented" — transfer completion says nothing about physical
//! presentation (finding 9).

use core::sync::atomic::{AtomicU32, Ordering};

use crate::geometry::FrameEpoch;
use crate::sweep::{AbortedSweep, Sweep, SweepPlan, SweepWritten};

/// A platform-supplied instant in platform-defined tick units. The time
/// source is trusted to be monotonic; written-settlement regressions are
/// clamped, but arbitrary forward values are outside this type's checks. The
/// finite domain ends at [`Tick::MAX`]; throttle addition never saturates.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Tick(pub u64);

impl Tick {
    /// The final instant representable by this profile-independent tick
    /// domain. A positive interval extending beyond it exhausts the demand
    /// machine's documented time horizon rather than becoming shorter.
    pub const MAX: Self = Self(u64::MAX);
}

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

// The id counter is u32 because thumbv7em-class targets have no 64-bit
// atomics (the bare-metal CI gate rejected AtomicU64). Exhaustion is
// handled EXPLICITLY rather than by silent wrap-aliasing (round-2
// finding): the 2^32nd construction aborts, which on any real deployment
// is unreachable and on a pathological one is the honest outcome. Ids are
// widened to u64 in witnesses; epochs are u64 and monotonic per demand,
// with the same documented exhaustion stance (2^64 sweeps).
static DEMAND_IDS: AtomicU32 = AtomicU32::new(0);

const EPOCH_EXHAUSTED: &str =
    "FrameDemand epoch space exhausted (2^64 sweep epochs minted; create a new demand machine)";
const TICK_HORIZON_EXHAUSTED: &str = "FrameDemand eligibility exceeds Tick::MAX (replace the demand machine after settling its epoch)";

fn mint_demand_id(counter: &AtomicU32) -> Option<u64> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .ok()
        .map(u64::from)
}

fn mint_demand_id_or_panic(counter: &AtomicU32) -> u64 {
    mint_demand_id(counter)
        .expect("FrameDemand provenance-id space exhausted (2^32 - 1 successful constructions)")
}

fn mint_epoch_or_panic(next_epoch: &mut Option<u64>) -> FrameEpoch {
    let raw = next_epoch.expect(EPOCH_EXHAUSTED);
    *next_epoch = raw.checked_add(1);
    FrameEpoch(raw)
}

fn eligibility_or_panic(last_written: Tick, min_interval: u64) -> Tick {
    Tick(
        last_written
            .0
            .checked_add(min_interval)
            .expect(TICK_HORIZON_EXHAUSTED),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InvalidationState {
    Clear,
    Pending,
    Active,
}

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
/// | any, sweeping | `begin_sweep` | `None` (one machine-active epoch) |
/// | sweeping | `request` | dirty (targets the next epoch, survives settlement) |
/// | sweeping | `finish_written` (active token, not invalidated) | idle, throttle → `now`, obligations cleared: `Effective` |
/// | sweeping | `finish_written` (active token, invalidated mid-sweep) | idle, dirty + full-repaint retained, throttle unchanged: `DiscardedByInvalidation` |
/// | any | `finish_written`/`finish_failed` (foreign/stale token) | `Err(ForeignSweep)`, **no mutation** |
/// | sweeping | `finish_failed` (active token) | idle, dirty retained, full-repaint set, throttle unchanged |
/// | sweeping | `abandon_active` | idle, dirty + full-repaint set, throttle unchanged (dropped-sweep recovery, finding 8) |
/// | sweeping | `invalidate` | dirty + full-repaint set; active sweep marked non-clearing |
/// | idle | `invalidate` | dirty + full-repaint set; sticky pending invalidation transfers to the next minted sweep |
#[derive(Debug)]
pub struct FrameDemand {
    id: u64,
    plan: SweepPlan,
    dirty: bool,
    sweeping: Option<FrameEpoch>,
    invalidation: InvalidationState,
    last_written: Option<Tick>,
    scheduled: Option<Tick>,
    min_interval: u64,
    next_epoch: Option<u64>,
    full_repaint: bool,
}

impl FrameDemand {
    /// Explicit throttle policy (tick units) and the fixed panel plan; no
    /// `Default`.
    /// # Panics
    ///
    /// Panics if `2^32 - 1` `FrameDemand` values have already been
    /// constructed in this program: provenance ids never alias; exhaustion
    /// is sticky even if a host catches the panic, rather than silently
    /// wrapping the counter.
    pub fn new(min_interval_ticks: u64, plan: SweepPlan) -> Self {
        let id = mint_demand_id_or_panic(&DEMAND_IDS);
        Self {
            id,
            plan,
            dirty: false,
            sweeping: None,
            invalidation: InvalidationState::Clear,
            last_written: None,
            scheduled: None,
            min_interval: min_interval_ticks,
            next_epoch: Some(0),
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
    /// written settlement is discarded rather than trusted (finding 7). If
    /// idle, the invalidation sticks until the next successful
    /// [`FrameDemand::begin_sweep`] transfers it into that minted sweep's
    /// discard state. Rejected, throttled, or panicking begin attempts cannot
    /// lose the latch. A finite private state, not a wrappable counter.
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.full_repaint = true;
        if self.sweeping.is_some() {
            self.invalidation = InvalidationState::Active;
        } else {
            self.invalidation = InvalidationState::Pending;
        }
    }

    /// Mints the sweep when one is due: demand pending, no machine-active
    /// epoch, throttle elapsed. Binds the caller-frozen snapshot, the fixed
    /// plan, the repaint mode, and the branded epoch. The throttled case
    /// records [`FrameDemand::eligible_at`]; calling this is also the sole
    /// acknowledgment of elapsed eligibility. Epochs 0 through `u64::MAX`
    /// are each minted once: exactly 2^64 successful sweeps.
    ///
    /// # Panics
    ///
    /// Panics before mutation if an eligible call would exceed the exact
    /// 2^64-minted-epoch horizon, or if `last_written + min_interval` exceeds
    /// [`Tick::MAX`]. Both are sticky finite-domain boundaries with fixed,
    /// build-profile-independent messages; neither counter wraps or
    /// saturates.
    pub fn begin_sweep<S>(&mut self, now: Tick, snapshot: S) -> Option<Sweep<S>> {
        if !self.dirty || self.sweeping.is_some() {
            return None;
        }
        if let Some(last) = self.last_written {
            let eligible = eligibility_or_panic(last, self.min_interval);
            if now < eligible {
                self.scheduled = Some(eligible);
                return None;
            }
        }
        let epoch = mint_epoch_or_panic(&mut self.next_epoch);
        let active_invalidated = self.invalidation == InvalidationState::Pending;
        self.dirty = false;
        self.scheduled = None;
        self.sweeping = Some(epoch);
        self.invalidation = if active_invalidated {
            InvalidationState::Active
        } else {
            InvalidationState::Clear
        };
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
    /// [`SweepWritten`]). `now` regressions are clamped: the throttle
    /// position never moves backward. `Tick` values are trusted platform
    /// input — the time source is a documented trust boundary, not a
    /// validated one (round-2 finding on forgeable time).
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
        if self.invalidation == InvalidationState::Active {
            // The panel changed under this sweep: output is suspect. Retain
            // demand and the obligation; do not advance the throttle.
            self.invalidation = InvalidationState::Clear;
            self.dirty = true;
            self.full_repaint = true;
            return Ok(WrittenDisposition::DiscardedByInvalidation);
        }
        self.last_written = Some(match self.last_written {
            Some(previous) if previous > now => previous, // clamp regression
            _ => now,
        });
        self.scheduled = None;
        self.full_repaint = false;
        Ok(WrittenDisposition::Effective)
    }

    /// Settles the active sweep as failed/aborted: demand retained, the
    /// full-repaint obligation recorded, the throttle not advanced. Because
    /// [`crate::sweep::Sweep::abort`] rejects outstanding work, an abort
    /// witness proves that no target or flight remains independently live.
    /// Calling [`FrameDemand::invalidate`] while idle still marks the next
    /// replacement non-clearing; its sticky latch cannot be erased by mint.
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
        self.invalidation = InvalidationState::Clear;
        self.dirty = true;
        self.full_repaint = true;
        Ok(())
    }

    /// Conservative recovery when the active sweep's value was dropped
    /// (early return, panic path, lost token): clears the active epoch,
    /// retains demand, forces a full repaint, does not advance the
    /// throttle. Idempotent when idle (finding 8).
    ///
    /// **Caller obligation (round-2 finding on premature abandonment):**
    /// this machine can terminally reject the abandoned epoch's witnesses
    /// (`ForeignSweep`), but it cannot stop a retained old `Sweep` from
    /// minting a target later. Drop every unstarted target and every old flight
    /// before beginning the replacement; a reviewed transfer adapter
    /// synchronously cancels/disarms on `Drop`, but safe Rust cannot force the
    /// caller to drop rather than retain or drive it. If that bounded stale
    /// write window can overlap the replacement, call
    /// [`FrameDemand::invalidate`] while idle: the sticky latch marks that next
    /// sweep non-clearing so another forced full repaint remains due.
    pub fn abandon_active(&mut self) {
        if self.sweeping.take().is_some() {
            self.invalidation = InvalidationState::Clear;
            self.dirty = true;
            self.full_repaint = true;
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    fn panic_message(payload: &(dyn core::any::Any + Send)) -> Option<&str> {
        payload.downcast_ref::<&str>().copied().or_else(|| {
            payload
                .downcast_ref::<std::string::String>()
                .map(std::string::String::as_str)
        })
    }

    #[test]
    fn demand_id_exhaustion_is_sticky() {
        let counter = AtomicU32::new(u32::MAX - 1);

        assert_eq!(mint_demand_id(&counter), Some(u64::from(u32::MAX - 1)));
        assert_eq!(counter.load(Ordering::Relaxed), u32::MAX);
        assert!(
            std::panic::catch_unwind(|| mint_demand_id_or_panic(&counter)).is_err(),
            "the exhausted constructor path panics"
        );
        assert_eq!(counter.load(Ordering::Relaxed), u32::MAX);
        assert_eq!(mint_demand_id(&counter), None, "exhaustion never reopens");
    }

    #[test]
    fn epoch_horizon_mints_max_once_then_panics_stickily() {
        let mut next = Some(u64::MAX);
        assert_eq!(mint_epoch_or_panic(&mut next), FrameEpoch(u64::MAX));
        assert_eq!(next, None, "MAX transitions to sticky exhaustion");

        for _ in 0..2 {
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                mint_epoch_or_panic(&mut next)
            }))
            .expect_err("a sweep beyond the full 2^64 horizon must panic");
            assert_eq!(
                panic_message(panic.as_ref()),
                Some(EPOCH_EXHAUSTED),
                "debug and release use the same explicit panic"
            );
            assert_eq!(next, None, "exhaustion remains sticky after unwind");
        }
    }

    #[test]
    fn eligibility_horizon_accepts_max_and_rejects_overflow() {
        assert_eq!(eligibility_or_panic(Tick(u64::MAX - 1), 1), Tick::MAX);
        let panic = std::panic::catch_unwind(|| eligibility_or_panic(Tick::MAX, 1))
            .expect_err("positive interval beyond MAX must not saturate");
        assert_eq!(panic_message(panic.as_ref()), Some(TICK_HORIZON_EXHAUSTED));
    }

    #[test]
    fn panicking_begin_preserves_pending_invalidation() {
        let plan = SweepPlan::for_panel(crate::geometry::PanelGeometry::WAVESHARE_18_V1, 448)
            .expect("anchor plan");

        let mut epoch_exhausted = FrameDemand::new(0, plan);
        epoch_exhausted.invalidate();
        epoch_exhausted.next_epoch = None;
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            epoch_exhausted.begin_sweep(Tick(0), ())
        }))
        .expect_err("epoch exhaustion must panic before mint");
        assert_eq!(panic_message(panic.as_ref()), Some(EPOCH_EXHAUSTED));
        assert_eq!(epoch_exhausted.invalidation, InvalidationState::Pending);
        assert!(epoch_exhausted.dirty);
        assert_eq!(epoch_exhausted.sweeping, None);

        let mut tick_exhausted = FrameDemand::new(1, plan);
        tick_exhausted.last_written = Some(Tick::MAX);
        tick_exhausted.invalidate();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tick_exhausted.begin_sweep(Tick::MAX, ())
        }))
        .expect_err("tick exhaustion must panic before mint");
        assert_eq!(panic_message(panic.as_ref()), Some(TICK_HORIZON_EXHAUSTED));
        assert_eq!(tick_exhausted.invalidation, InvalidationState::Pending);
        assert!(tick_exhausted.dirty);
        assert_eq!(tick_exhausted.sweeping, None);
    }
}
