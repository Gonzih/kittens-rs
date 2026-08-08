//! Client submissions (SPEC rule P1).
//!
//! `resume` is deliberately not an op: resuming is a driver startup mode
//! (open log → validate epoch → crash-repair → replay), not a wire message.

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::config::SessionConfigPatch;
use crate::ids::SubmissionId;
use crate::policy::ApprovalVerdict;

/// One client request. Fork, rewind, and swarm mount ops are reserved
/// post-KC0 shapes and intentionally absent here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum Op {
    /// Start or continue the conversation with user text.
    UserInput {
        /// The user's message.
        text: String,
    },
    /// Mid-turn user message, delivered without a deferral instruction.
    Interject {
        /// The interjection text.
        text: String,
    },
    /// Answer an approval request.
    Approve {
        /// The approval request being answered.
        request: SubmissionId,
        /// The decision.
        verdict: ApprovalVerdict,
    },
    /// Abort the in-flight turn without ending the session.
    Interrupt,
    /// Patch the session configuration; accepted patches are logged.
    ConfigPatch {
        /// The partial update.
        patch: SessionConfigPatch,
    },
    /// End the session: stop starting effects, drain, close the log.
    Shutdown,
}

/// An op with its correlation id.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Submission {
    /// Client-chosen correlation id echoed by responding events.
    pub id: SubmissionId,
    /// The request.
    pub op: Op,
}
