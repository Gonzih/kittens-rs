//! Deterministic token accounting (SPEC rule C8, decision D-a).
//!
//! The driving number for compaction triggers is hybrid: provider-reported
//! usage for history (exact) plus a tail estimator self-calibrated each
//! turn against that same provider usage. All arithmetic is integer
//! fixed-point so the jail's byte-identical replay guarantee (gate G2)
//! holds across targets — no floats anywhere near a behavioral decision.
//!
//! The estimator also tracks its own observed error bounds; gate G5 reports
//! them next to the E1 results so compaction-trigger noise is quantified.

/// Fixed-point scale for the bytes-per-token ratio (millitokens).
const RATIO_SCALE: u64 = 1_000;

/// Fallback ratio before any calibration sample exists: 4000 milli-bytes
/// per token (the bytes/4 folk heuristic, held only until the first real
/// provider report replaces it).
const DEFAULT_BYTES_PER_TOKEN_MILLI: u64 = 4_000;

/// Hybrid token accounting for one session.
#[derive(Clone, Debug)]
pub struct TokenAccounting {
    /// Exact prompt tokens the provider reported for committed history.
    history_tokens: u64,
    /// Calibrated ratio: bytes per token, scaled by [`RATIO_SCALE`].
    bytes_per_token_milli: u64,
    /// Calibration samples observed.
    samples: u32,
    /// Worst observed relative error of pre-report estimates, in permille.
    max_error_permille: u32,
}

impl Default for TokenAccounting {
    fn default() -> Self {
        Self {
            history_tokens: 0,
            bytes_per_token_milli: DEFAULT_BYTES_PER_TOKEN_MILLI,
            samples: 0,
            max_error_permille: 0,
        }
    }
}

impl TokenAccounting {
    /// Estimates tokens for `bytes` of not-yet-reported tail content.
    ///
    /// All products are computed in `u128` and saturated back to `u64` so no
    /// adversarial byte count can overflow (review input 19 #15); the result
    /// is deterministic across targets.
    #[must_use]
    pub fn estimate_tail(&self, bytes: u64) -> u64 {
        let scaled = u128::from(bytes) * u128::from(RATIO_SCALE);
        let per = u128::from(self.bytes_per_token_milli.max(1));
        u64::try_from(scaled.div_ceil(per)).unwrap_or(u64::MAX)
    }

    /// The current best window total: exact history plus estimated tail.
    #[must_use]
    pub fn window_tokens(&self, tail_bytes: u64) -> u64 {
        self.history_tokens
            .saturating_add(self.estimate_tail(tail_bytes))
    }

    /// Records a provider usage report covering content that measured
    /// `reported_bytes` on our side and `reported_tokens` on the provider's.
    ///
    /// Recalibrates the ratio (running mean over samples) and records the
    /// relative error the pre-report estimate would have made. All
    /// intermediate products use `u128` so no input can overflow.
    pub fn record_provider_usage(&mut self, reported_tokens: u64, reported_bytes: u64) {
        if reported_tokens == 0 {
            return;
        }
        let estimate = self.estimate_tail(reported_bytes);
        let error = u128::from(estimate.abs_diff(reported_tokens));
        let error_permille = u32::try_from((error * 1_000).div_ceil(u128::from(reported_tokens)))
            .unwrap_or(u32::MAX);
        self.max_error_permille = self.max_error_permille.max(error_permille);

        let observed_milli =
            (u128::from(reported_bytes) * u128::from(RATIO_SCALE)) / u128::from(reported_tokens);
        let n = u128::from(self.samples);
        let blended = ((u128::from(self.bytes_per_token_milli) * n) + observed_milli) / (n + 1);
        self.bytes_per_token_milli = u64::try_from(blended).unwrap_or(u64::MAX).max(1);
        self.samples = self.samples.saturating_add(1);
        self.history_tokens = self.history_tokens.saturating_add(reported_tokens);
    }

    /// Exact provider-reported history tokens accumulated so far.
    #[must_use]
    pub fn history_tokens(&self) -> u64 {
        self.history_tokens
    }

    /// Resets the exact history counter after compaction rebuilt the window
    /// (the calibrated ratio survives; it describes the model, not the
    /// window).
    pub fn reset_history(&mut self, new_history_tokens: u64) {
        self.history_tokens = new_history_tokens;
    }

    /// Worst observed relative estimator error, in permille (G5 reporting).
    #[must_use]
    pub fn max_error_permille(&self) -> u32 {
        self.max_error_permille
    }

    /// Calibration samples observed (G5 reporting).
    #[must_use]
    pub fn samples(&self) -> u32 {
        self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ratio_is_bytes_over_four() {
        let acc = TokenAccounting::default();
        assert_eq!(acc.estimate_tail(4_000), 1_000);
    }

    #[test]
    fn calibration_moves_the_ratio() {
        let mut acc = TokenAccounting::default();
        // Provider says 1000 tokens for 3000 bytes: 3 bytes/token. The
        // first real sample replaces the bytes/4 heuristic entirely
        // (samples counts real reports only).
        acc.record_provider_usage(1_000, 3_000);
        assert_eq!(acc.history_tokens(), 1_000);
        assert_eq!(acc.estimate_tail(3_000), 1_000);
    }

    #[test]
    fn error_bounds_are_tracked() {
        let mut acc = TokenAccounting::default();
        acc.record_provider_usage(1_000, 3_000);
        // Estimate for 3000 bytes at default ratio was 750: 25% error.
        assert_eq!(acc.max_error_permille(), 250);
    }

    #[test]
    fn extreme_inputs_do_not_overflow() {
        let mut acc = TokenAccounting::default();
        // u64::MAX bytes at the default ratio must compute without panicking
        // (u128 intermediate) and stay within u64.
        let est = acc.estimate_tail(u64::MAX);
        assert!(est > 0);
        // Provider reports at the extremes must not panic.
        acc.record_provider_usage(u64::MAX, u64::MAX);
        acc.record_provider_usage(1, u64::MAX);
        // With a tiny ratio and huge tail, the tail estimate saturates to
        // the ceiling rather than wrapping.
        acc.bytes_per_token_milli = 1;
        assert_eq!(acc.estimate_tail(u64::MAX), u64::MAX);
        let _ = acc.window_tokens(u64::MAX);
    }

    #[test]
    fn deterministic_across_runs() {
        let mut a = TokenAccounting::default();
        let mut b = TokenAccounting::default();
        for (t, y) in [(100, 380), (220, 700), (57, 231)] {
            a.record_provider_usage(t, y);
            b.record_provider_usage(t, y);
        }
        assert_eq!(a.estimate_tail(12_345), b.estimate_tail(12_345));
        assert_eq!(a.max_error_permille(), b.max_error_permille());
    }
}
