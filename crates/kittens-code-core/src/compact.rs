//! Compaction decision engine (SPEC rules C1–C4; gates G5, G6).
//!
//! Pure decision logic: given the current window pressure and its own
//! state, it says what the context engine should do next. It never touches
//! records — compaction rebuilds the *window view* and never deletes log
//! content (C1). Escalation order is microcompact → full summary →
//! mechanical drop-oldest, with prefire scheduling (background summary
//! started below the hard trigger) and a circuit breaker that suppresses
//! repeated deterministic failure (C2/C3).

use kittens_code_protocol::config::CompactionThresholds;

/// Deterministic failures tolerated before the breaker opens.
const BREAKER_LIMIT: u8 = 2;

/// What the context engine should do now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompactionDecision {
    /// Nothing; pressure is below every threshold.
    None,
    /// Start background prefire summarization (emit the sub-model effect).
    StartPrefire,
    /// Age stale tool results out of the window (cheap, cache-preserving).
    Microcompact,
    /// Apply the ready summary: rebuild the window via `WindowLayout`.
    ApplySummary,
    /// No summary is ready at the hard line: drop oldest turns mechanically.
    DropOldest,
    /// The breaker is open; compaction is suppressed until config changes.
    Suppressed,
}

/// Compaction scheduling state for one session.
///
/// The four booleans are independent latch bits of one scheduling protocol,
/// not a disguised enum: prefire can be in flight while a previous summary
/// is still unapplied, and the breaker is orthogonal to both.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CompactionState {
    prefire_inflight: bool,
    summary_ready: bool,
    microcompact_done_at_pressure: bool,
    failures: u8,
    suppressed: bool,
}

impl CompactionState {
    /// Decides the next action for `window_percent` of context used.
    ///
    /// Call once per turn boundary (between-turn trigger path); the
    /// mid-turn hard-overflow and provider-400 recovery paths reuse the
    /// same decisions with `window_percent = 100`.
    #[must_use]
    pub fn decide(
        &mut self,
        window_percent: u8,
        thresholds: &CompactionThresholds,
    ) -> CompactionDecision {
        if self.suppressed {
            return CompactionDecision::Suppressed;
        }
        if window_percent >= thresholds.hard_percent {
            if self.summary_ready {
                return CompactionDecision::ApplySummary;
            }
            if !self.microcompact_done_at_pressure {
                self.microcompact_done_at_pressure = true;
                return CompactionDecision::Microcompact;
            }
            return CompactionDecision::DropOldest;
        }
        if window_percent >= thresholds.prefire_percent {
            if !self.prefire_inflight && !self.summary_ready {
                self.prefire_inflight = true;
                return CompactionDecision::StartPrefire;
            }
            return CompactionDecision::None;
        }
        // Pressure receded below prefire: reset the per-pressure microcompact
        // marker so the next climb gets its cheap tier again.
        self.microcompact_done_at_pressure = false;
        CompactionDecision::None
    }

    /// The prefire summarization effect finished successfully.
    pub fn prefire_succeeded(&mut self) {
        self.prefire_inflight = false;
        self.summary_ready = true;
        self.failures = 0;
    }

    /// The prefire summarization effect failed.
    ///
    /// Deterministic repeated failure opens the breaker (C2).
    pub fn prefire_failed(&mut self) {
        self.prefire_inflight = false;
        self.failures = self.failures.saturating_add(1);
        if self.failures >= BREAKER_LIMIT {
            self.suppressed = true;
        }
    }

    /// A summary was applied; the window was rebuilt.
    pub fn summary_applied(&mut self) {
        self.summary_ready = false;
        self.microcompact_done_at_pressure = false;
    }

    /// Configuration changed: close the breaker and forget failures.
    pub fn reset_breaker(&mut self) {
        self.failures = 0;
        self.suppressed = false;
    }

    /// Whether the breaker is currently open (event reporting).
    #[must_use]
    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> CompactionThresholds {
        CompactionThresholds::default()
    }

    #[test]
    fn quiet_below_prefire() {
        let mut s = CompactionState::default();
        assert_eq!(s.decide(50, &thresholds()), CompactionDecision::None);
    }

    #[test]
    fn prefire_fires_once() {
        let mut s = CompactionState::default();
        assert_eq!(
            s.decide(76, &thresholds()),
            CompactionDecision::StartPrefire
        );
        assert_eq!(s.decide(78, &thresholds()), CompactionDecision::None);
    }

    #[test]
    fn hard_line_prefers_ready_summary() {
        let mut s = CompactionState::default();
        assert_eq!(
            s.decide(76, &thresholds()),
            CompactionDecision::StartPrefire
        );
        s.prefire_succeeded();
        assert_eq!(
            s.decide(86, &thresholds()),
            CompactionDecision::ApplySummary
        );
        s.summary_applied();
        assert_eq!(s.decide(50, &thresholds()), CompactionDecision::None);
    }

    #[test]
    fn hard_line_without_summary_escalates() {
        let mut s = CompactionState::default();
        assert_eq!(
            s.decide(90, &thresholds()),
            CompactionDecision::Microcompact
        );
        assert_eq!(s.decide(90, &thresholds()), CompactionDecision::DropOldest);
    }

    #[test]
    fn breaker_opens_after_repeated_failure_and_resets() {
        let mut s = CompactionState::default();
        assert_eq!(
            s.decide(76, &thresholds()),
            CompactionDecision::StartPrefire
        );
        s.prefire_failed();
        assert_eq!(
            s.decide(76, &thresholds()),
            CompactionDecision::StartPrefire
        );
        s.prefire_failed();
        assert_eq!(s.decide(90, &thresholds()), CompactionDecision::Suppressed);
        s.reset_breaker();
        assert_eq!(
            s.decide(76, &thresholds()),
            CompactionDecision::StartPrefire
        );
    }
}
