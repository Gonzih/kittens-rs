//! Sealed, kind-branded budget cap-types (SPEC rules P6, Q3; gate G3).
//!
//! Every byte of RLM-originated data and every tool result that reaches the
//! root window travels as a [`Capped`] value. The only constructors truncate;
//! no `Deserialize` impl exists; the window-insertion APIs accept nothing
//! else. Bypassing this module is therefore a compile error, which is what
//! gate G3's trybuild fixtures prove.
//!
//! Cap kinds are sealed: the set of budget-bearing categories is spec law,
//! not an extension point. Each kind carries a compile-time hard ceiling;
//! runtime limits from `SessionConfig` are clamped to it.

use alloc::format;
use alloc::string::String;
use core::marker::PhantomData;

use kittens_code_protocol::budgets::BudgetKind;

mod sealed {
    pub trait Sealed {}
}

/// A sealed budget category with its compile-time hard ceiling.
pub trait CapKind: sealed::Sealed {
    /// The absolute byte ceiling no runtime configuration can exceed.
    const HARD_CEILING: usize;
    /// The budget meter this kind reports under.
    const KIND: BudgetKind;
}

/// Per-verb RLM output surfaced to the root window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerbOutput {}
/// Per-tool-result output surfaced to the root window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolResult {}
/// Per-`ask` sub-model digest surfaced to the root window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AskDigest {}

impl sealed::Sealed for VerbOutput {}
impl sealed::Sealed for ToolResult {}
impl sealed::Sealed for AskDigest {}

impl CapKind for VerbOutput {
    const HARD_CEILING: usize = 65_536;
    const KIND: BudgetKind = BudgetKind::VerbOutput;
}
impl CapKind for ToolResult {
    const HARD_CEILING: usize = 65_536;
    const KIND: BudgetKind = BudgetKind::ToolResult;
}
impl CapKind for AskDigest {
    const HARD_CEILING: usize = 32_768;
    const KIND: BudgetKind = BudgetKind::AskDigest;
}

/// What truncation removed, kept so the model can recover the remainder
/// through the log (reversible offload, SPEC Q3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Truncation {
    /// Byte length of the original value.
    pub original_bytes: u64,
    /// Bytes kept from the head.
    pub head_bytes: u32,
    /// Bytes kept from the tail (zero for head-only truncation).
    pub tail_bytes: u32,
    /// Log sequence where the full value lives, when it was recorded.
    pub log_seq: Option<u64>,
}

/// A budget-capped text value; the only path into the root window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capped<K: CapKind> {
    text: String,
    applied_limit: u32,
    truncation: Option<Truncation>,
    _kind: PhantomData<K>,
}

/// Clamps a UTF-8 boundary downward from `at` (which must be `<= len`).
fn floor_char_boundary(s: &str, mut at: usize) -> usize {
    while at > 0 && !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Clamps a UTF-8 boundary upward from `at` (which must be `<= len`).
fn ceil_char_boundary(s: &str, mut at: usize) -> usize {
    while at < s.len() && !s.is_char_boundary(at) {
        at += 1;
    }
    at
}

impl<K: CapKind> Capped<K> {
    /// Effective limit: the runtime limit clamped to the hard ceiling.
    fn effective_limit(runtime_limit: u32) -> usize {
        (runtime_limit as usize).min(K::HARD_CEILING)
    }

    /// Truncates keeping only the head of the value.
    ///
    /// `log_seq` names the record holding the full value when one exists;
    /// truncation is reversible offload, never loss (SPEC Q3).
    #[must_use]
    pub fn head(value: &str, runtime_limit: u32, log_seq: Option<u64>) -> Self {
        let limit = Self::effective_limit(runtime_limit);
        if value.len() <= limit {
            return Self {
                text: String::from(value),
                applied_limit: runtime_limit,
                truncation: None,
                _kind: PhantomData,
            };
        }
        let head_end = floor_char_boundary(value, limit);
        Self {
            text: String::from(&value[..head_end]),
            applied_limit: runtime_limit,
            truncation: Some(Truncation {
                original_bytes: value.len() as u64,
                head_bytes: u32::try_from(head_end).unwrap_or(u32::MAX),
                tail_bytes: 0,
                log_seq,
            }),
            _kind: PhantomData,
        }
    }

    /// Truncates keeping an excerpt from both ends (the tool-result shape:
    /// head context plus the freshest tail, SPEC Q3).
    #[must_use]
    pub fn head_tail(value: &str, runtime_limit: u32, log_seq: Option<u64>) -> Self {
        let limit = Self::effective_limit(runtime_limit);
        if value.len() <= limit {
            return Self {
                text: String::from(value),
                applied_limit: runtime_limit,
                truncation: None,
                _kind: PhantomData,
            };
        }
        let head_budget = limit / 2;
        let tail_budget = limit - head_budget;
        let head_end = floor_char_boundary(value, head_budget);
        let tail_start = ceil_char_boundary(value, value.len() - tail_budget);
        let mut text = String::with_capacity(limit);
        text.push_str(&value[..head_end]);
        text.push_str(&value[tail_start..]);
        Self {
            text,
            applied_limit: runtime_limit,
            truncation: Some(Truncation {
                original_bytes: value.len() as u64,
                head_bytes: u32::try_from(head_end).unwrap_or(u32::MAX),
                tail_bytes: u32::try_from(value.len() - tail_start).unwrap_or(u32::MAX),
                log_seq,
            }),
            _kind: PhantomData,
        }
    }

    /// The capped text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The runtime limit that was applied (pre-clamp value as declared).
    #[must_use]
    pub fn applied_limit(&self) -> u32 {
        self.applied_limit
    }

    /// Truncation metadata; `None` when the value fit whole.
    #[must_use]
    pub fn truncation(&self) -> Option<&Truncation> {
        self.truncation.as_ref()
    }

    /// The budget meter this value reports under.
    #[must_use]
    pub fn budget_kind() -> BudgetKind {
        K::KIND
    }
}

impl Capped<ToolResult> {
    /// Builds the canonical reversible-offload tool result.
    ///
    /// Values that exceed `runtime_limit` retain head and tail context plus
    /// a log pointer, with the annotation itself included inside the branded
    /// cap. This keeps the `TailItem` type boundary honest: no uncapped
    /// annotation is appended after construction.
    #[must_use]
    pub fn tool_result(value: &str, runtime_limit: u32, log_seq: u64) -> Self {
        let limit = Self::effective_limit(runtime_limit);
        if value.len() <= limit {
            return Self::head_tail(value, runtime_limit, Some(log_seq));
        }

        let annotation = format!(
            "\n[truncated from {} bytes; full output at log seq {log_seq}]",
            value.len()
        );
        let excerpt_limit = limit.saturating_sub(annotation.len());
        let excerpt_limit = u32::try_from(excerpt_limit).unwrap_or(u32::MAX);
        let excerpt = Self::head_tail(value, excerpt_limit, Some(log_seq));
        let mut text = excerpt.text;
        let annotation_end = floor_char_boundary(&annotation, limit.saturating_sub(text.len()));
        text.push_str(&annotation[..annotation_end]);
        Self {
            text,
            applied_limit: runtime_limit,
            truncation: excerpt.truncation,
            _kind: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values_pass_untouched() {
        let c = Capped::<VerbOutput>::head("hello", 8_192, None);
        assert_eq!(c.as_str(), "hello");
        assert!(c.truncation().is_none());
    }

    #[test]
    fn head_truncates_on_char_boundary() {
        // 'é' is two bytes; a 5-byte limit must not split it.
        let c = Capped::<VerbOutput>::head("aaaaé!", 5, Some(42));
        assert_eq!(c.as_str(), "aaaa");
        let t = c.truncation().expect("truncated");
        assert_eq!(t.original_bytes, 7);
        assert_eq!(t.head_bytes, 4);
        assert_eq!(t.tail_bytes, 0);
        assert_eq!(t.log_seq, Some(42));
    }

    #[test]
    fn head_tail_keeps_both_ends() {
        let value = "HEAD-xxxxxxxxxxxxxxxxxxxx-TAIL";
        let c = Capped::<ToolResult>::head_tail(value, 12, Some(7));
        assert!(c.as_str().starts_with("HEAD-"));
        assert!(c.as_str().ends_with("-TAIL"));
        assert!(c.as_str().len() <= 12);
        let t = c.truncation().expect("truncated");
        assert_eq!(t.original_bytes, value.len() as u64);
        assert!(t.tail_bytes > 0);
    }

    #[test]
    fn runtime_limit_clamps_to_hard_ceiling() {
        let big = "a".repeat(70_000);
        let c = Capped::<VerbOutput>::head(&big, u32::MAX, None);
        assert_eq!(c.as_str().len(), 65_536);
        assert!(c.truncation().is_some());
    }

    #[test]
    fn tool_result_annotation_stays_inside_brand() {
        let value = "x".repeat(1_000);
        let capped = Capped::<ToolResult>::tool_result(&value, 96, 42);
        assert!(capped.as_str().len() <= 96);
        assert!(capped.as_str().contains("full output at log seq 42"));
        assert_eq!(capped.applied_limit(), 96);
        assert_eq!(capped.truncation().unwrap().original_bytes, 1_000);
    }
}
