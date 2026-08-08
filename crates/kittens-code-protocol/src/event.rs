//! Harness events (SPEC rule P2).
//!
//! Authoritative events are derived from committed records and published
//! only after the covering durability watermark; model deltas may
//! additionally arrive early with `preview: true` and are reconciled by
//! record sequence (SPEC L-A3).

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::budgets::BudgetKind;
use crate::error::ErrorEvent;
use crate::ids::{EffectId, SubmissionId, TurnEpoch};

/// Why a turn ended.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum TurnEnd {
    /// The model produced a terminal response with no tool calls.
    Completed,
    /// The operator interrupted the turn.
    Interrupted,
    /// The turn died on a fatal error (details arrive as an error event).
    Failed,
}

/// How a tool call finished.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum ToolOutcome {
    /// Ran to completion; the (truncated) result went to the model.
    Succeeded,
    /// Ran and failed.
    Failed {
        /// Short failure description.
        message: String,
    },
    /// Denied by policy or operator verdict.
    Denied,
    /// Aborted by interrupt, shutdown, or deadline.
    Aborted,
}

/// One harness event on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum Event {
    /// A turn began.
    TurnStarted {
        /// The new turn's epoch.
        epoch: TurnEpoch,
        /// The submission that started it, when one did.
        correlates: Option<SubmissionId>,
    },
    /// A turn ended.
    TurnEnded {
        /// The turn's epoch.
        epoch: TurnEpoch,
        /// Why it ended.
        reason: TurnEnd,
    },
    /// A span of model output text.
    ModelDelta {
        /// The producing turn.
        epoch: TurnEpoch,
        /// `true` for pre-durability preview copies; the authoritative
        /// copy follows with the same `record_seq`.
        preview: bool,
        /// The record this delta belongs to, for reconciliation.
        record_seq: u64,
        /// The text span.
        text: String,
    },
    /// The model proposed a tool call (pre-approval).
    ToolProposed {
        /// Correlates the call across its lifecycle.
        call: EffectId,
        /// Tool name.
        name: String,
        /// JSON-encoded arguments as produced by the model.
        args_json: String,
    },
    /// An approved tool call started executing.
    ToolStarted {
        /// The call.
        call: EffectId,
    },
    /// A span of tool output (already budget-truncated for the window).
    ToolOutputDelta {
        /// The call.
        call: EffectId,
        /// The output span.
        chunk: String,
    },
    /// A tool call finished; exactly one per call.
    ToolTerminal {
        /// The call.
        call: EffectId,
        /// How it finished.
        outcome: ToolOutcome,
    },
    /// The harness asks the operator to approve a tool call.
    ApprovalRequested {
        /// Answer with [`crate::op::Op::Approve`] naming this id.
        request: SubmissionId,
        /// The call awaiting the verdict.
        call: EffectId,
        /// Human-readable description of what would run.
        description: String,
    },
    /// A budget meter advanced or was resized.
    BudgetUpdate {
        /// Which meter.
        kind: BudgetKind,
        /// Amount consumed so far.
        used: u64,
        /// The declared limit.
        limit: u64,
    },
    /// Background prefire summarization began.
    CompactionStarted {
        /// The turn during which it was scheduled.
        epoch: TurnEpoch,
    },
    /// A compacted window layout was applied.
    CompactionApplied {
        /// The turn whose window was rebuilt.
        epoch: TurnEpoch,
    },
    /// Compaction was suppressed by the circuit breaker.
    CompactionSuppressed {
        /// The turn during which suppression triggered.
        epoch: TurnEpoch,
    },
    /// One RLM query line executed (trace-level visibility).
    QueryTrace {
        /// The query.
        query: EffectId,
        /// One-based line number within the query script.
        line: u16,
        /// Human-readable note (verb, meters charged, error binding).
        note: String,
    },
    /// An error.
    Error(ErrorEvent),
    /// The session is draining toward exit.
    ShuttingDown,
}
