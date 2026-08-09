//! Typed intermediate representation for RLM query scripts.

use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use kittens_code_protocol::error::VerbErrorCause;
use serde::{Deserialize, Serialize};

/// The result type declared by an RLM instruction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Out {
    /// A selection of transcript records.
    Records,
    /// A list of partitioned transcript selections.
    Chunks,
    /// An integer count.
    Count,
    /// One capped sub-model digest.
    Digest,
    /// Capped sub-model digests, ordered by partition index.
    DigestList,
    /// The query's final answer.
    Answer,
}

/// Marker type for a reference whose output must be [`Out::Records`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Records {}

/// Marker type for a reference whose output must be [`Out::Chunks`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Chunks {}

/// Marker type for a reference allowed to name any successful output.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Any {}

/// A one-based, backward reference to a typed query result slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ref<T> {
    /// One-based line/slot number, as written by `%N`.
    line: u32,
    #[serde(skip)]
    marker: PhantomData<fn() -> T>,
}

impl<T> Ref<T> {
    /// Constructs a typed reference to a one-based result slot.
    #[must_use]
    pub const fn new(line: u32) -> Self {
        Self {
            line,
            marker: PhantomData,
        }
    }

    /// Returns the one-based result slot named by this reference.
    #[must_use]
    pub const fn line(self) -> u32 {
        self.line
    }
}

/// The unit attached to an inclusive-start, exclusive-end selection range.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeUnit {
    /// User-turn indices.
    Turn,
    /// Transcript record sequence numbers.
    Seq,
    /// Offsets in the store byte view.
    Byte,
}

/// An inclusive-start, exclusive-end transcript selection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Range {
    /// The coordinate system of `start` and `end`.
    pub unit: RangeUnit,
    /// The included lower bound.
    pub start: u64,
    /// The excluded upper bound.
    pub end: u64,
}

/// A transcript selection used by record-reading instructions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "selection", content = "value")]
pub enum Sel {
    /// Records produced by an earlier line.
    Ref(Ref<Records>),
    /// A coordinate range in the transcript.
    Range(Range),
    /// The whole transcript, used when the surface omits a selector.
    Whole,
}

/// The partitioning strategy for [`Instr::Partition`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum By {
    /// Split on user-turn boundaries.
    Turns,
    /// Split after a byte count.
    Bytes,
    /// Split at matches of a regular-expression pattern.
    Regex,
}

/// An open event-kind name used by the `grep --kind` filter.
///
/// The protocol publishes an open [`kittens_code_protocol::event::Event`]
/// enum rather than a separate closed event-kind type, so the IR retains the
/// textual kind without guessing at future protocol variants.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct EventKind(String);

impl EventKind {
    /// Constructs an event-kind name after surface validation.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self(name)
    }

    /// Returns the preserved event-kind spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The literal-or-reference input accepted by [`Instr::Final`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "value", content = "data")]
pub enum FinalValue {
    /// A literal answer.
    Literal(String),
    /// Any successful output from an earlier line.
    Ref(Ref<Any>),
}

/// One instruction in the closed KC0 RLM IR.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "instruction")]
pub enum Instr {
    /// Search selected records for a pattern, with optional context and kind.
    Grep {
        /// Versioned-dialect regular-expression pattern.
        pattern: String,
        /// Records to search.
        sel: Sel,
        /// Number of surrounding records to include.
        ctx: u16,
        /// Optional event-kind restriction.
        kind: Option<EventKind>,
    },
    /// Select records without transforming them.
    Slice {
        /// Records or coordinate range to select.
        sel: Sel,
    },
    /// Select the first `n` records.
    Head {
        /// Records to read.
        sel: Sel,
        /// Maximum record count.
        n: u32,
    },
    /// Select the last `n` records.
    Tail {
        /// Records to read.
        sel: Sel,
        /// Maximum record count.
        n: u32,
    },
    /// Count all selected records or only pattern matches.
    Count {
        /// Optional versioned-dialect regular-expression pattern.
        pattern: Option<String>,
        /// Records to count.
        sel: Sel,
    },
    /// Divide a selection into chunks.
    Partition {
        /// Records to divide.
        sel: Sel,
        /// Partitioning strategy.
        by: By,
        /// Chunk size, required for turn/byte partitioning.
        size: Option<u32>,
        /// Separator pattern, required for regex partitioning.
        pattern: Option<String>,
    },
    /// Ask a sub-model one question over a selection.
    Ask {
        /// Records supplied to the sub-model.
        sel: Sel,
        /// Question supplied to the sub-model.
        question: String,
        /// Optional deterministic sample count.
        sample_k: Option<u8>,
    },
    /// Ask the same question independently over each earlier chunk.
    AskEach {
        /// A chunk-list output from an earlier `partition` line.
        chunks: Ref<Chunks>,
        /// Question supplied for every chunk.
        question: String,
    },
    /// Terminate the query with a literal or earlier value.
    Final {
        /// The answer source.
        value: FinalValue,
    },
}

impl Instr {
    /// Returns the output type declared by this instruction variant.
    #[must_use]
    pub const fn output(&self) -> Out {
        match self {
            Self::Grep { .. } | Self::Slice { .. } | Self::Head { .. } | Self::Tail { .. } => {
                Out::Records
            }
            Self::Count { .. } => Out::Count,
            Self::Partition { .. } => Out::Chunks,
            Self::Ask { .. } => Out::Digest,
            Self::AskEach { .. } => Out::DigestList,
            Self::Final { .. } => Out::Answer,
        }
    }
}

/// An inline error value bound to a query result slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerbError {
    /// Best-effort first token from the line, empty only when none exists.
    pub verb: String,
    /// Protocol-defined reason for the failure.
    pub cause: VerbErrorCause,
}

/// The value bound by one nonempty text-surface line.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "binding", content = "value")]
pub enum BoundValue {
    /// A successfully lowered instruction.
    Instr(Instr),
    /// A Q9 inline verb error; later typed references cannot use this slot.
    Error(VerbError),
}

/// One one-based result-slot binding produced from a nonempty script line.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Binding {
    /// One-based result slot assigned to the line.
    pub slot: u32,
    /// The instruction or inline error value bound to the slot.
    pub value: BoundValue,
}

impl Binding {
    /// Returns this binding's declared output, or `None` for an error value.
    #[must_use]
    pub fn output(&self) -> Option<Out> {
        match &self.value {
            BoundValue::Instr(instruction) => Some(instruction.output()),
            BoundValue::Error(_) => None,
        }
    }
}

/// A lowered query in source order, with one binding per nonempty line.
pub type Query = Vec<Binding>;
