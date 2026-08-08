//! Budget numbers as wire data (SPEC rules P6 and Q5).
//!
//! These are the *declared limits*; enforcement lives in the core's sealed
//! cap-types and meters, never here. Every field is a runtime limit that
//! must sit at or below the core's compile-time hard ceiling for its kind.

use serde::{Deserialize, Serialize};

/// Which budget a [`crate::error::ErrorCode::BudgetExhausted`] names.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    /// Per-verb output bytes surfaced to the root window.
    VerbOutput,
    /// Per-tool-result bytes surfaced to the root window.
    ToolResult,
    /// Per-`ask` digest bytes surfaced to the root window.
    AskDigest,
    /// Verbs per RLM query.
    VerbCount,
    /// RLM recursion depth.
    RecursionDepth,
    /// Total sub-model calls per query.
    TotalSubcalls,
    /// Concurrently running sub-model calls per query.
    ParallelSubcalls,
    /// Chunks produced by one `partition`.
    PartitionCount,
    /// Bytes selected into sub-model calls per query.
    SelectedBytes,
    /// Store pages scanned per query.
    ScannedPages,
    /// Store bytes scanned per query.
    ScannedBytes,
    /// Store page effects issued per query.
    PageEffects,
    /// Simultaneously suspended RLM queries per session.
    SuspendedQueries,
    /// Aggregate retained continuation memory per session.
    ContinuationMemory,
    /// Wall-clock milliseconds per `ask` node.
    AskWallClock,
    /// Tokens per `ask` node.
    AskTokens,
    /// Turn-level token budget.
    TurnTokens,
}

/// The declared budget set for one session (`SessionConfig` data).
///
/// Value caps (`verb_output_bytes`, `tool_result_bytes`, `ask_digest_bytes`)
/// truncate and never error; aggregate meters error when exhausted
/// (SPEC P8 cap/meter rule).
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Budgets {
    /// Per-verb output cap in bytes (deliberate bytes-not-chars deviation).
    pub verb_output_bytes: u32,
    /// Per-tool-result root-window cap in bytes.
    pub tool_result_bytes: u32,
    /// Per-`ask` digest cap in bytes.
    pub ask_digest_bytes: u32,
    /// Maximum verbs in one RLM query.
    pub verb_count: u16,
    /// RLM recursion depth (economics cap; default 1).
    pub recursion_depth: u8,
    /// Total sub-model calls per query.
    pub total_subcalls: u16,
    /// Parallel sub-model call window per query.
    pub parallel_subcalls: u8,
    /// Maximum chunks from one `partition`.
    pub partition_count: u16,
    /// Selected-bytes ceiling per query.
    pub selected_bytes: u32,
    /// Scanned-pages ceiling per query.
    pub scanned_pages: u32,
    /// Scanned-bytes ceiling per query.
    pub scanned_bytes: u64,
    /// Page-effect ceiling per query.
    pub page_effects: u32,
    /// Simultaneously suspended queries per session.
    pub suspended_queries: u8,
    /// Aggregate retained continuation memory per session, in bytes.
    pub continuation_memory_bytes: u32,
    /// Wall-clock milliseconds per `ask` node.
    pub ask_wall_clock_ms: u32,
    /// Token ceiling per `ask` node.
    pub ask_tokens: u32,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            verb_output_bytes: 8_192,
            tool_result_bytes: 8_192,
            ask_digest_bytes: 4_096,
            verb_count: 64,
            recursion_depth: 1,
            total_subcalls: 32,
            parallel_subcalls: 4,
            partition_count: 256,
            selected_bytes: 1_048_576,
            scanned_pages: 4_096,
            scanned_bytes: 67_108_864,
            page_effects: 4_096,
            suspended_queries: 4,
            continuation_memory_bytes: 4_194_304,
            ask_wall_clock_ms: 120_000,
            ask_tokens: 32_768,
        }
    }
}
