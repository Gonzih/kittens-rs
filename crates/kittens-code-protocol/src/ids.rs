//! Identity types (SPEC rule P9): plain arrays and integers, no `uuid` or
//! `semver` dependencies, no wall-time requirement.

use serde::{Deserialize, Serialize};

/// Session identity: sixteen driver-generated random bytes.
///
/// Drivers generate this from their entropy source at session creation; the
/// core never manufactures one. No UUID version semantics are implied — a
/// microcontroller without trusted wall time produces these as legitimately
/// as a desktop.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SessionId(pub [u8; 16]);

/// Correlates one driver-side unit of work (an effect) across its start,
/// progress, terminal, and cancellation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectId(pub u64);

/// Monotonic turn identity. Every effect and completion carries the epoch it
/// belongs to; completions from an aborted epoch are dropped with a trace.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct TurnEpoch(pub u64);

/// Correlates a client [`crate::op::Op`] with the events it caused.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SubmissionId(pub u64);

/// A content digest as plain bytes; the hashing algorithm is declared by the
/// producing context, not by this type.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Digest(pub [u8; 32]);

/// A `major.minor.patch` version as plain integers (used for the prompt
/// pack, verb grammar, and L3 dialect versions recorded in the log header).
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct VersionTriple(pub [u16; 3]);
