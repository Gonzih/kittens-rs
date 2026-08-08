//! The patchable session configuration (SPEC rules P5 and F4).
//!
//! `SessionConfig` is the *logged, replayable* half of configuration:
//! budgets, thresholds, prompt overrides, symbolic model profile ids, and
//! policy defaults. Precedence is defaults < file < accepted
//! `config_patch` op, last-wins per leaf; every accepted patch becomes a
//! log record, so configuration state replays with the session.
//!
//! Bootstrap configuration — endpoints, auth, TLS roots, store paths,
//! preopens, flash partitions — is deliberately absent: it is driver-only
//! and never enters the wire or the log.

use alloc::collections::BTreeMap;
use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::budgets::Budgets;
use crate::policy::{ApprovalPolicy, SandboxPolicy};

/// Live-window compaction thresholds as percentages of the context window.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CompactionThresholds {
    /// Background prefire summarization starts here.
    pub prefire_percent: u8,
    /// The hard compaction trigger.
    pub hard_percent: u8,
}

impl Default for CompactionThresholds {
    fn default() -> Self {
        Self {
            prefire_percent: 75,
            hard_percent: 85,
        }
    }
}

/// Doom-loop stationarity guard thresholds (SPEC L-T3).
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StationarityThresholds {
    /// Identical consecutive tool calls before the guard fires.
    pub identical_calls: u16,
    /// Identical consecutive no-op calls before the guard fires.
    pub identical_noops: u16,
}

impl Default for StationarityThresholds {
    fn default() -> Self {
        Self {
            identical_calls: 16,
            identical_noops: 4,
        }
    }
}

/// Driver queue bounds (SPEC L-A2); capacity law lives in the driver.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct QueueBounds {
    /// Model output delta queue depth.
    pub model_deltas: u16,
    /// Effect progress queue depth.
    pub effect_progress: u16,
    /// Interjection queue depth.
    pub interjections: u16,
    /// Maximum concurrently running effects.
    pub max_active_effects: u16,
}

impl Default for QueueBounds {
    fn default() -> Self {
        Self {
            model_deltas: 256,
            effect_progress: 128,
            interjections: 16,
            max_active_effects: 16,
        }
    }
}

/// The full patchable session configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SessionConfig {
    /// Compaction scheduling thresholds.
    pub compaction: CompactionThresholds,
    /// Stationarity guard thresholds.
    pub stationarity: StationarityThresholds,
    /// Declared budget limits.
    pub budgets: Budgets,
    /// Driver queue bounds.
    pub queues: QueueBounds,
    /// Prompt-pack overrides keyed by template id (SPEC C9).
    pub prompt_overrides: BTreeMap<String, String>,
    /// Symbolic root-model profile id, resolved by bootstrap config.
    pub model_root: String,
    /// Symbolic sub-model profile id, resolved by bootstrap config.
    pub model_sub: String,
    /// Per-tool approval defaults keyed by tool name.
    pub approval_defaults: BTreeMap<String, ApprovalPolicy>,
    /// Default sandbox policy; a per-turn op-supplied policy overrides it.
    pub sandbox_default: Option<SandboxPolicy>,
}

/// A partial update to [`SessionConfig`]; `None` leaves a leaf untouched.
///
/// Patches are applied last-wins per leaf and recorded in the log.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SessionConfigPatch {
    /// Replace compaction thresholds.
    pub compaction: Option<CompactionThresholds>,
    /// Replace stationarity thresholds.
    pub stationarity: Option<StationarityThresholds>,
    /// Replace the budget set.
    pub budgets: Option<Budgets>,
    /// Replace queue bounds.
    pub queues: Option<QueueBounds>,
    /// Merge prompt overrides (per-key last-wins).
    pub prompt_overrides: Option<BTreeMap<String, String>>,
    /// Replace the root-model profile id.
    pub model_root: Option<String>,
    /// Replace the sub-model profile id.
    pub model_sub: Option<String>,
    /// Merge approval defaults (per-key last-wins).
    pub approval_defaults: Option<BTreeMap<String, ApprovalPolicy>>,
    /// Replace the default sandbox policy.
    pub sandbox_default: Option<SandboxPolicy>,
}

impl SessionConfig {
    /// Applies a patch, last-wins per leaf; map-valued leaves merge per key.
    pub fn apply(&mut self, patch: SessionConfigPatch) {
        if let Some(v) = patch.compaction {
            self.compaction = v;
        }
        if let Some(v) = patch.stationarity {
            self.stationarity = v;
        }
        if let Some(v) = patch.budgets {
            self.budgets = v;
        }
        if let Some(v) = patch.queues {
            self.queues = v;
        }
        if let Some(v) = patch.prompt_overrides {
            self.prompt_overrides.extend(v);
        }
        if let Some(v) = patch.model_root {
            self.model_root = v;
        }
        if let Some(v) = patch.model_sub {
            self.model_sub = v;
        }
        if let Some(v) = patch.approval_defaults {
            self.approval_defaults.extend(v);
        }
        if let Some(v) = patch.sandbox_default {
            self.sandbox_default = Some(v);
        }
    }
}
