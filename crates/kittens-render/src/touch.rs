//! The touch service protocol: generation-latched, snapshot-read, bounded
//! per activation, honestly coalescing.
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
//! - [`TouchGenerations`]: the shared produced/serviced pair. The ISR side
//!   does exactly one thing — bump the produced generation (and wakes
//!   through its platform slot; the wake mechanism itself is the
//!   platform's, per the K2R-0A completion verdict). Wrap safety comes from
//!   comparing generations by equality, never by ordering.
//! - [`TouchService`]: the pure, deterministic service machine — bounded
//!   snapshot reads per activation, re-service on generation change or
//!   asserted INT, pending state restored on read failure. Being pure, its
//!   interleavings are tested exhaustively without threads.
//! - [`TouchDelta`]: pure edge reconstruction between surfaced reports.

use core::sync::atomic::{AtomicU32, Ordering};

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
    /// The contact appears in both; position may have changed.
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
                Some(now) => ContactEdge::Moved {
                    from: *point,
                    to: now,
                },
                None => ContactEdge::Up(*point),
            };
            edges[slot] = Some(edge);
            slot += 1;
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

/// The shared produced/serviced generation pair.
///
/// The ISR side calls [`TouchGenerations::produce`]; the service side reads
/// and marks. Both sides use wrapping increments and compare only by
/// equality, so a `u32` wrap is harmless (finding 12's wrap concern).
#[derive(Debug, Default)]
pub struct TouchGenerations {
    produced: AtomicU32,
    serviced: AtomicU32,
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
        }
    }

    /// ISR side: records one interrupt activation. Returns `true` when the
    /// pair transitioned from idle to pending — the caller wakes its
    /// platform slot exactly then (wake dedup, not wake suppression:
    /// additional produces while already pending are captured by the
    /// generation, not by extra wakes).
    pub fn produce(&self) -> bool {
        let was_pending = self.is_pending();
        self.produced.fetch_add(1, Ordering::AcqRel);
        !was_pending
    }

    /// Whether an unserviced generation exists.
    pub fn is_pending(&self) -> bool {
        self.produced.load(Ordering::Acquire) != self.serviced.load(Ordering::Acquire)
    }

    fn produced_now(&self) -> u32 {
        self.produced.load(Ordering::Acquire)
    }

    fn mark_serviced(&self, generation: u32) {
        self.serviced.store(generation, Ordering::Release);
    }
}

/// Platform boundary for one snapshot read. The FT3168 integration reads
/// one contiguous register block per call and parses count/event/id/
/// coordinates from that single block — never from separate transactions.
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Activation {
    /// All pending generations serviced; INT deasserted; the source may go
    /// quiescent until the next produce.
    Idle {
        /// Snapshots surfaced during this activation.
        surfaced: u8,
    },
    /// The per-activation budget was exhausted while work remained (stuck
    /// INT line, or a producer outrunning the budget). The caller MUST
    /// re-arm/re-schedule; the reactor is not monopolized.
    BudgetExhausted {
        /// Snapshots surfaced during this activation.
        surfaced: u8,
    },
    /// A snapshot read failed. Pending state is restored: the failed
    /// generation will be serviced by the next activation.
    ReadFailed {
        /// Snapshots surfaced before the failure.
        surfaced: u8,
    },
}

/// The pure, bounded touch service machine.
#[derive(Debug)]
pub struct TouchService {
    budget_per_activation: u8,
    last_surfaced: TouchReport,
}

impl TouchService {
    /// Explicit per-activation snapshot budget; no `Default`. One or two is
    /// typical: the budget bounds reactor monopolization, not input rate.
    pub const fn new(budget_per_activation: u8) -> Self {
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
    /// Protocol per SPEC 5.4: service while a generation is pending or INT
    /// is asserted, at most `budget_per_activation` snapshot reads; a
    /// generation produced *during* a read is observed by the post-read
    /// generation check and serviced within budget; a failed read restores
    /// pending state and ends the activation.
    pub fn service<R: TouchReader>(
        &mut self,
        generations: &TouchGenerations,
        reader: &mut R,
        mut surface: impl FnMut(TouchReport, TouchDelta),
    ) -> Activation {
        let mut surfaced: u8 = 0;
        loop {
            let pending = generations.is_pending() || reader.int_asserted();
            if !pending {
                return Activation::Idle { surfaced };
            }
            if surfaced >= self.budget_per_activation {
                return Activation::BudgetExhausted { surfaced };
            }

            let generation_at_start = generations.produced_now();
            match reader.read_snapshot() {
                Ok(report) => {
                    let delta = TouchDelta::between(&self.last_surfaced, &report);
                    self.last_surfaced = report;
                    surface(report, delta);
                    // The snapshot covers every produce up to the read
                    // start; later produces stay pending by generation
                    // inequality and drive another loop turn.
                    generations.mark_serviced(generation_at_start);
                    surfaced += 1;
                }
                Err(_) => {
                    // Nothing marked: the pending generation survives for
                    // the next activation.
                    return Activation::ReadFailed { surfaced };
                }
            }
        }
    }
}
