//! The touch service protocol: generation-counted, pending-latched,
//! snapshot-read, bounded per activation, honestly coalescing.
//!
//! Semantics (SPEC 5.4, review findings 11/12): **latest-state with
//! coalescing** — every surfaced report is one complete, untorn snapshot;
//! intermediate reports may coalesce; down/up edges are reconstructed as
//! deltas between *surfaced* reports. A contact that begins and ends
//! entirely between two snapshots is physically unobservable and no queue
//! pretends otherwise.
//!
//! Division of labor:
//!
//! - [`TouchGenerations`]: the shared produced/serviced pair plus an
//!   authoritative pending/retry latch. The ISR side increments first,
//!   then latches and wakes only on the latch's idle-to-pending transition.
//!   The service side claims the latch before each read, so arrivals during
//!   the read remain visible even if the generation counter wraps.
//! - [`TouchService`]: the deterministic service machine — bounded
//!   snapshot reads per activation, re-service on generation change or
//!   asserted INT, pending state restored on read failure. Being pure, its
//!   adversarial interleavings are tested deterministically without threads.
//! - [`TouchDelta`]: pure edge reconstruction between surfaced reports.

use core::num::NonZeroU8;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// One tracked contact. The FT3168 reports at most two.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TouchPoint {
    /// Controller-assigned contact identity.
    pub id: u8,
    /// Panel X coordinate.
    pub x: u16,
    /// Panel Y coordinate.
    pub y: u16,
}

/// One complete, untorn touch snapshot. Fixed two-point capacity bounds
/// simultaneous contacts (the controller's own limit), not temporal
/// backlog — which this protocol honestly does not keep.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TouchReport {
    /// Contacts present in this snapshot.
    pub points: [Option<TouchPoint>; 2],
}

impl TouchReport {
    fn point_by_id(&self, id: u8) -> Option<TouchPoint> {
        self.points
            .iter()
            .flatten()
            .find(|point| point.id == id)
            .copied()
    }
}

/// Per-contact edges reconstructed between two *surfaced* reports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContactEdge {
    /// The contact appears in `next` but not `prev`.
    Down(TouchPoint),
    /// The contact appears in both and the complete point changed.
    Moved {
        /// Position in the previous surfaced report.
        from: TouchPoint,
        /// Position in the new surfaced report.
        to: TouchPoint,
    },
    /// The contact appears in `prev` but not `next`.
    Up(TouchPoint),
}

/// Edge reconstruction between two surfaced reports.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TouchDelta {
    /// At most two prev-contacts plus two next-contacts can change state.
    pub edges: [Option<ContactEdge>; 4],
}

impl TouchDelta {
    /// Reconstructs per-id edges between two surfaced reports.
    pub fn between(prev: &TouchReport, next: &TouchReport) -> Self {
        let mut edges = [None; 4];
        let mut slot = 0;
        // Ups and moves: walk previous contacts.
        for point in prev.points.iter().flatten() {
            let edge = match next.point_by_id(point.id) {
                Some(now) if now != *point => Some(ContactEdge::Moved {
                    from: *point,
                    to: now,
                }),
                Some(_) => None,
                None => Some(ContactEdge::Up(*point)),
            };
            if let Some(edge) = edge {
                edges[slot] = Some(edge);
                slot += 1;
            }
        }
        // Downs: next contacts absent from previous.
        for point in next.points.iter().flatten() {
            if prev.point_by_id(point.id).is_none() {
                edges[slot] = Some(ContactEdge::Down(*point));
                slot += 1;
            }
        }
        Self { edges }
    }
}

/// Shared touch-arrival state for one producer domain and one service
/// consumer.
///
/// `produced`/`serviced` detect an arrival concurrent with a snapshot, but
/// the separate `pending` latch is authoritative for scheduling and retry.
/// Counter equality therefore never means idle by itself: after any
/// positive multiple of `2^32` unserviced produces the counters may alias,
/// while the latch remains set.
///
/// The platform registers/arms its wake destination before enabling the
/// interrupt that calls [`TouchGenerations::produce`]. This ordering is
/// required because the latch's false-to-true transition may be the only
/// wake for a coalesced run of produces. Exactly one service consumer may
/// call [`TouchService::service`] at a time.
#[derive(Debug, Default)]
pub struct TouchGenerations {
    produced: AtomicU32,
    serviced: AtomicU32,
    pending: AtomicBool,
}

impl TouchGenerations {
    /// Creates an idle pair (nothing produced, nothing pending).
    pub const fn new() -> Self {
        Self::new_at(0)
    }

    /// Creates an idle pair with both counters seeded at `generation`.
    /// Exists so tests can exercise the wrap boundary for real; production
    /// code uses [`TouchGenerations::new`].
    #[doc(hidden)]
    pub const fn new_at(generation: u32) -> Self {
        Self {
            produced: AtomicU32::new(generation),
            serviced: AtomicU32::new(generation),
            pending: AtomicBool::new(false),
        }
    }

    /// Creates a pending state whose counters have aliased at `generation`.
    ///
    /// This is the state reached after a positive multiple of `2^32`
    /// produces without a successful snapshot. It exists for deterministic
    /// wrap-boundary oracles; production code uses
    /// [`TouchGenerations::new`].
    #[doc(hidden)]
    pub const fn new_pending_at(generation: u32) -> Self {
        Self {
            produced: AtomicU32::new(generation),
            serviced: AtomicU32::new(generation),
            pending: AtomicBool::new(true),
        }
    }

    /// ISR side: records one touch arrival and reports whether to wake the
    /// registered platform slot.
    ///
    /// The wrapping generation increment is sequenced before
    /// `pending.swap(true)`. **Why:** a consumer racing the handoff must
    /// either observe the new generation before it clears/rechecks, or the
    /// swap must observe a cleared latch and request a wake. The caller
    /// wakes exactly when this returns `true`; `false` means a service or
    /// retry is already latched, so another wake would be redundant.
    #[must_use = "a true return is the only wake request for this arrival; ignoring it strands latched work"]
    pub fn produce(&self) -> bool {
        self.produce_with_after_increment(|| {})
    }

    /// Whether service or retry is authoritatively latched.
    ///
    /// This reads the persistent latch, never generation equality. The latch
    /// remains authoritative across INT-only work, read failure, budget
    /// exhaustion, and generation wrap. The sole consumer temporarily
    /// clears it only while owning a snapshot attempt, and restores it before
    /// returning from a failed attempt.
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire)
    }

    // The callback is a deterministic oracle seam at the protocol's only
    // two-atomic producer boundary. The public path supplies an empty
    // callback, so production and the interleaving oracle execute the same
    // operations in the same order.
    fn produce_with_after_increment(&self, after_increment: impl FnOnce()) -> bool {
        self.produced.fetch_add(1, Ordering::Relaxed);
        after_increment();
        !self.pending.swap(true, Ordering::AcqRel)
    }

    fn latch_retry(&self) {
        self.pending.store(true, Ordering::Release);
    }

    fn claim_snapshot(&self) -> u32 {
        // Clear BEFORE sampling the generation and beginning the read.
        // WHY: every producer during the read must independently set the
        // latch; a wrapping counter is then corroboration, never the only
        // evidence of concurrent work. The acquire half observes the
        // generation published before a producer's release swap.
        self.pending.swap(false, Ordering::AcqRel);
        self.produced.load(Ordering::Acquire)
    }

    fn complete_snapshot(&self, generation: u32, int_asserted: bool) {
        self.serviced.store(generation, Ordering::Release);
        let generation_changed =
            self.produced.load(Ordering::Acquire) != self.serviced.load(Ordering::Acquire);
        let arrival_latched = self.pending.load(Ordering::Acquire);
        if generation_changed || arrival_latched || int_asserted {
            // Re-latch BEFORE another loop turn can report budget
            // exhaustion. WHY: BudgetExhausted and ReadFailed rely on this
            // latch, rather than a future interrupt edge, for retry.
            self.latch_retry();
        }
    }
}

/// Platform boundary for one snapshot read. A reviewed FT3168 integration
/// MUST read one contiguous register block per call and parse count/event/
/// id/coordinates from that single block — never from separate
/// transactions. This crate does not yet contain that concrete integration.
pub trait TouchReader {
    /// Transport error type.
    type Error;

    /// Reads one complete, untorn snapshot.
    ///
    /// # Errors
    ///
    /// The transport error; the service machine restores pending state so
    /// no generation is consumed by a failed read.
    fn read_snapshot(&mut self) -> Result<TouchReport, Self::Error>;

    /// Whether the interrupt line is currently asserted.
    fn int_asserted(&self) -> bool;
}

/// Outcome of one bounded service activation.
#[must_use = "BudgetExhausted and ReadFailed require the caller to re-arm; ignoring the outcome strands latched work"]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Activation {
    /// The final handoff observed no pending work and INT deasserted; the
    /// source may go quiescent. A producer racing after that observation
    /// latches work and requests a wake.
    Idle {
        /// Snapshots surfaced during this activation.
        surfaced: u8,
    },
    /// The per-activation budget was exhausted while work remained (stuck
    /// INT line, or a producer outrunning the budget). Pending/retry remains
    /// latched. The caller MUST re-arm/re-schedule this source; later
    /// producers may deduplicate their wake against that latch.
    BudgetExhausted {
        /// Snapshots surfaced during this activation.
        surfaced: u8,
    },
    /// A snapshot read failed. Pending/retry remains latched, including for
    /// an activation started by INT alone. The caller MUST re-arm/re-schedule
    /// this source; the next activation retries even if INT deasserts.
    ReadFailed {
        /// Snapshots surfaced before the failure.
        surfaced: u8,
    },
}

/// The deterministic, bounded touch service machine.
#[derive(Debug)]
pub struct TouchService {
    budget_per_activation: NonZeroU8,
    last_surfaced: TouchReport,
}

impl TouchService {
    /// Creates a service with an explicit, nonzero snapshot budget.
    ///
    /// One or two is typical: the budget bounds reactor monopolization, not
    /// input rate. [`NonZeroU8`] makes a permanent zero-progress service
    /// unrepresentable; there is deliberately no `Default`.
    pub const fn new(budget_per_activation: NonZeroU8) -> Self {
        Self {
            budget_per_activation,
            last_surfaced: TouchReport {
                points: [None, None],
            },
        }
    }

    /// The most recently surfaced report (the "latest state").
    pub const fn latest(&self) -> TouchReport {
        self.last_surfaced
    }

    /// Runs one bounded service activation, surfacing each complete
    /// snapshot (with its edges relative to the previously surfaced report)
    /// through `surface`.
    ///
    /// Protocol per SPEC 5.4: service while pending/retry is latched or INT
    /// is asserted, at most the configured number of snapshot reads. Before
    /// each read the consumer clears the latch, then samples the generation;
    /// after a successful read it rechecks the latch, generation, and INT
    /// and re-latches if any says more work exists. **Why this order:** an
    /// arrival during the read sets the cleared latch even across exact
    /// counter wrap, while an arrival after the final check sets the latch
    /// and requests its own wake. A failed read re-latches before returning.
    ///
    /// `BudgetExhausted` and `ReadFailed` leave the latch set. Their caller
    /// must arrange another activation rather than waiting for another
    /// [`TouchGenerations::produce`] call, whose wake may be deduplicated.
    pub fn service<R: TouchReader>(
        &mut self,
        generations: &TouchGenerations,
        reader: &mut R,
        mut surface: impl FnMut(TouchReport, TouchDelta),
    ) -> Activation {
        let mut surfaced: u8 = 0;
        loop {
            if reader.int_asserted() {
                generations.latch_retry();
            }
            if !generations.is_pending() {
                return Activation::Idle { surfaced };
            }
            if surfaced >= self.budget_per_activation.get() {
                return Activation::BudgetExhausted { surfaced };
            }

            let generation_at_start = generations.claim_snapshot();
            if let Ok(report) = reader.read_snapshot() {
                let delta = TouchDelta::between(&self.last_surfaced, &report);
                self.last_surfaced = report;
                surface(report, delta);
                surfaced += 1;
                generations.complete_snapshot(generation_at_start, reader.int_asserted());
            } else {
                // The claim cleared the latch before the fallible read;
                // restore it before returning so INT-only work and a
                // now-deasserted line still get a retry.
                generations.latch_retry();
                return Activation::ReadFailed { surfaced };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OneSnapshot {
        reads: u8,
        fail_next: bool,
        int_asserted: bool,
    }

    impl TouchReader for OneSnapshot {
        type Error = ();

        fn read_snapshot(&mut self) -> Result<TouchReport, Self::Error> {
            self.reads += 1;
            if self.fail_next {
                self.fail_next = false;
                Err(())
            } else {
                Ok(TouchReport::default())
            }
        }

        fn int_asserted(&self) -> bool {
            self.int_asserted
        }
    }

    fn service_one_snapshot(
        service: &mut TouchService,
        generations: &TouchGenerations,
        reader: &mut OneSnapshot,
    ) -> Activation {
        // Keep the success and failure traces in one concrete service
        // instantiation so they exercise the same protocol machine.
        service.service(generations, reader, |_, _| {})
    }

    #[test]
    fn increment_then_latch_closes_idle_check_lost_wake() {
        let generations = TouchGenerations::new();
        assert!(generations.produce(), "initial transition requests a wake");

        let mut service = TouchService::new(NonZeroU8::MIN);
        let mut reader = OneSnapshot {
            reads: 0,
            fail_next: false,
            int_asserted: false,
        };
        let mut activation = None;

        // Pause the second producer after its generation increment but
        // before its latch swap. The consumer can complete and exit idle;
        // the producer must then see the cleared latch and request a wake.
        let wake = generations.produce_with_after_increment(|| {
            activation = Some(service_one_snapshot(
                &mut service,
                &generations,
                &mut reader,
            ));
        });

        assert_eq!(activation, Some(Activation::Idle { surfaced: 1 }));
        assert_eq!(reader.reads, 1);
        assert!(wake, "post-idle latch transition requests the wake");
        assert!(
            generations.is_pending(),
            "the requested activation remains latched"
        );

        // The producer won the post-idle latch transition. If the immediate
        // retry read fails, that exact work must remain authoritative even
        // after INT deasserts, and the following activation must recover it.
        reader.fail_next = true;
        assert_eq!(
            service_one_snapshot(&mut service, &generations, &mut reader),
            Activation::ReadFailed { surfaced: 0 }
        );
        assert_eq!(reader.reads, 2);
        assert!(
            generations.is_pending(),
            "the failed handoff read restores retry authority"
        );
        assert_eq!(
            service_one_snapshot(&mut service, &generations, &mut reader),
            Activation::Idle { surfaced: 1 }
        );
        assert_eq!(reader.reads, 3);
        assert!(
            !generations.is_pending(),
            "a successful retry completes the handoff"
        );

        // Exercise the same concrete service machine with INT stuck asserted:
        // one complete snapshot consumes the budget, re-latches work, and the
        // next loop turn must yield without monopolizing the reactor.
        reader.int_asserted = true;
        assert_eq!(
            service_one_snapshot(&mut service, &generations, &mut reader),
            Activation::BudgetExhausted { surfaced: 1 }
        );
        assert_eq!(reader.reads, 4);
        assert!(
            generations.is_pending(),
            "budget exhaustion preserves retry authority while INT is asserted"
        );

        reader.int_asserted = false;
        assert_eq!(
            service_one_snapshot(&mut service, &generations, &mut reader),
            Activation::Idle { surfaced: 1 }
        );
        assert_eq!(reader.reads, 5);
        assert!(
            !generations.is_pending(),
            "the latched stuck-INT retry clears after INT deasserts"
        );
    }
}
