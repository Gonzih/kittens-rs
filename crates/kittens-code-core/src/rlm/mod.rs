//! Closed RLM instruction representation and versioned text lowering.

/// Closed, typed instruction representation shared by RLM surfaces.
pub mod ir;
/// Appendix-A text-surface lowering into the typed representation.
pub mod lower;

pub use ir::{
    Any, Binding, BoundValue, By, Chunks, EventKind, FinalValue, Instr, Out, Query, Range,
    RangeUnit, Records, Ref, Sel, VerbError,
};
pub use lower::{lower_script, lower_script_with_verb_limit};
