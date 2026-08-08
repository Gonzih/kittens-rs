//! Approval and sandbox policy as data (SPEC rule P4).
//!
//! Mechanism — how a sandbox is actually erected, how an approval prompt is
//! shown — is driver territory. The protocol carries only the declared
//! policy so it can travel on the wire, live in `SessionConfig`, and be
//! replayed from the log.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Default approval behavior for one tool family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// Execute without asking.
    Auto,
    /// Round-trip an approval request to the client first.
    Ask,
    /// Refuse without asking.
    Deny,
}

/// The verdict a client returns for one approval request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ApprovalVerdict {
    /// Run it.
    Approve,
    /// Refuse it; the tool call terminates as denied.
    Deny,
}

/// Declared filesystem/process authority for tool execution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum SandboxPolicy {
    /// No sandbox restrictions (explicitly dangerous; never a default).
    DangerFullAccess,
    /// Reads permitted, all mutations refused.
    ReadOnly,
    /// Mutations confined to the listed mount-relative roots.
    WorkspaceWrite {
        /// Mount-relative directories writes may touch.
        writable_roots: Vec<String>,
    },
}
