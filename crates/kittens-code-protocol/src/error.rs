//! Error taxonomy (SPEC rule P8, decision D10).

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::budgets::BudgetKind;
use crate::ids::SubmissionId;

/// How a caller should treat an error. The class is data shipped with the
/// code (see [`ErrorCode::class`]), never caller judgment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    /// Transient; the same request may succeed later.
    Retryable,
    /// The session cannot usefully continue on this path.
    Fatal,
    /// A human decision or configuration change is required.
    UserActionable,
}

/// Why an RLM verb line failed (binds inline to its `%N` slot; SPEC Q9).
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum VerbErrorCause {
    /// A `%N` reference names no earlier line or the wrong output type.
    BadRef,
    /// A range is inverted, out of unit bounds, or malformed.
    BadRange,
    /// An unknown or duplicate flag, or a flag value of the wrong shape.
    BadFlag,
    /// The line does not parse under the versioned grammar.
    Parse,
    /// An in-script aggregate meter was exhausted (value caps truncate
    /// instead and never raise this).
    Budget,
}

/// The closed KC0 error code set. Additive-only within v0.x.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case", tag = "code")]
pub enum ErrorCode {
    /// Transport-level model call failure (connect, TLS, stream drop).
    ModelTransport,
    /// The provider reported overload or rate limiting.
    ModelOverloaded,
    /// The provider rejected the request for context length.
    ModelContextLength,
    /// Authentication or authorization failure against the provider.
    ModelAuth,
    /// A tool call was denied by policy or the operator.
    ToolDenied,
    /// A tool ran and failed.
    ToolFailed,
    /// A tool exceeded its deadline.
    ToolTimeout,
    /// A query- or turn-level budget was exhausted.
    BudgetExhausted {
        /// Which budget ran out.
        budget_kind: BudgetKind,
    },
    /// An RLM verb line failed; bound inline to its `%N` slot.
    VerbError {
        /// The verb that failed.
        verb: String,
        /// Why it failed.
        cause: VerbErrorCause,
    },
    /// A persisted log's `schema_epoch` is newer than this binary supports.
    SchemaIncompatible,
    /// The store failed to append or read durably.
    StoreIo,
    /// A configuration file or patch was rejected.
    ConfigInvalid,
    /// The operation was cancelled by interrupt or shutdown.
    Cancelled,
    /// An invariant the spec promises was violated; always a bug.
    Internal,
}

impl ErrorCode {
    /// The class shipped with each code (SPEC P8: data, not judgment).
    #[must_use]
    pub fn class(&self) -> ErrorClass {
        match self {
            Self::ModelTransport
            | Self::ModelOverloaded
            | Self::ModelContextLength
            | Self::BudgetExhausted { .. }
            | Self::VerbError { .. }
            | Self::ToolFailed
            | Self::ToolTimeout
            | Self::Cancelled => ErrorClass::Retryable,
            Self::SchemaIncompatible | Self::StoreIo | Self::Internal => ErrorClass::Fatal,
            Self::ModelAuth | Self::ToolDenied | Self::ConfigInvalid => ErrorClass::UserActionable,
        }
    }
}

/// One error on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ErrorEvent {
    /// Caller treatment class (redundant with [`ErrorCode::class`] so
    /// clients need not embed the mapping).
    pub class: ErrorClass,
    /// The specific code, with its payload.
    pub code: ErrorCode,
    /// Human-readable context; never required for programmatic handling.
    pub message: String,
    /// The client submission this error answers, when one exists.
    pub correlates: Option<SubmissionId>,
}

impl ErrorEvent {
    /// Builds an event with the class derived from the code.
    #[must_use]
    pub fn new(code: ErrorCode, message: String, correlates: Option<SubmissionId>) -> Self {
        Self {
            class: code.class(),
            code,
            message,
            correlates,
        }
    }
}
