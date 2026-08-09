//! The typed post-compaction window layout (SPEC rules C10, P5-adjacent;
//! decision D4).
//!
//! The layout is a fixed, ordered recipe — `[system, user_info,
//! rules_reminder, last_user_query, verbatim_tail, summary, reminders]` —
//! and its constructor enforces the tail-atomicity invariant: a tool call
//! and its terminal are never split across the compaction boundary (gate
//! G6 fuzzes exactly this). Segments carry semantic region labels for
//! optional serving-layer co-design; no KC0 behavior depends on them.

use alloc::string::String;
use alloc::vec::Vec;

use kittens_code_protocol::ids::EffectId;

/// Semantic region labels (SPEC section 5.1 serving co-design).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Region {
    /// Static system prompt prefix (cache-stable).
    System,
    /// Environment/user information block.
    UserInfo,
    /// Re-injected rules content (escaped, C7).
    RulesReminder,
    /// The last real user query, verbatim.
    LastUserQuery,
    /// Verbatim tail since the last real user turn.
    VerbatimTail,
    /// The model-generated compaction summary.
    Summary,
    /// Mutable-state reminder blocks (C5).
    Reminders,
}

/// One item of the verbatim tail, kept at message granularity so the
/// constructor can check call/terminal pairing.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TailItem {
    /// An ordinary message span (user, assistant, or reminder text).
    Message(String),
    /// A tool call the model issued.
    ToolCall {
        /// The call's effect identity.
        call: EffectId,
        /// Rendered call content.
        text: String,
    },
    /// A tool result (already budget-truncated upstream).
    ToolResult {
        /// The call this result answers.
        call: EffectId,
        /// Rendered (capped) result content.
        text: String,
    },
}

/// Why a layout was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LayoutError {
    /// A tail tool result appears without its call earlier in the tail —
    /// the compaction boundary split an atomic pair (SPEC P5/S3 law).
    TailSplitsToolPair,
}

/// The typed window recipe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowLayout {
    /// Static system prompt ([`Region::System`]).
    pub system: String,
    /// Environment block ([`Region::UserInfo`]).
    pub user_info: String,
    /// Escaped rules content ([`Region::RulesReminder`]).
    pub rules_reminder: String,
    /// The last real user query ([`Region::LastUserQuery`]).
    pub last_user_query: String,
    /// Verbatim tail since the last real user turn ([`Region::VerbatimTail`]).
    pub verbatim_tail: Vec<TailItem>,
    /// Compaction summary, empty before first compaction ([`Region::Summary`]).
    pub summary: String,
    /// Reminder blocks ([`Region::Reminders`]).
    pub reminders: Vec<String>,
}

impl WindowLayout {
    /// Builds a layout, enforcing tail atomicity.
    ///
    /// # Errors
    ///
    /// [`LayoutError::TailSplitsToolPair`] when any `ToolResult` in the
    /// tail lacks its `ToolCall` earlier in the same tail.
    pub fn new(
        system: String,
        user_info: String,
        rules_reminder: String,
        last_user_query: String,
        verbatim_tail: Vec<TailItem>,
        summary: String,
        reminders: Vec<String>,
    ) -> Result<Self, LayoutError> {
        let mut seen_calls: Vec<EffectId> = Vec::new();
        for item in &verbatim_tail {
            match item {
                TailItem::ToolCall { call, .. } => seen_calls.push(*call),
                TailItem::ToolResult { call, .. } => {
                    if !seen_calls.contains(call) {
                        return Err(LayoutError::TailSplitsToolPair);
                    }
                }
                TailItem::Message(_) => {}
            }
        }
        Ok(Self {
            system,
            user_info,
            rules_reminder,
            last_user_query,
            verbatim_tail,
            summary,
            reminders,
        })
    }

    /// The fixed region order of the recipe (for emitters that label
    /// segments for the serving layer).
    #[must_use]
    pub fn region_order() -> [Region; 7] {
        [
            Region::System,
            Region::UserInfo,
            Region::RulesReminder,
            Region::LastUserQuery,
            Region::VerbatimTail,
            Region::Summary,
            Region::Reminders,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn s(v: &str) -> String {
        String::from(v)
    }

    #[test]
    fn paired_tail_is_accepted() {
        let tail = vec![
            TailItem::Message(s("assistant text")),
            TailItem::ToolCall {
                call: EffectId(1),
                text: s("read foo.rs"),
            },
            TailItem::ToolResult {
                call: EffectId(1),
                text: s("contents"),
            },
        ];
        assert!(WindowLayout::new(s("sys"), s("env"), s(""), s("q"), tail, s(""), vec![]).is_ok());
    }

    #[test]
    fn orphan_result_is_refused() {
        let tail = vec![TailItem::ToolResult {
            call: EffectId(9),
            text: s("orphaned"),
        }];
        assert_eq!(
            WindowLayout::new(s("sys"), s("env"), s(""), s("q"), tail, s(""), vec![]).unwrap_err(),
            LayoutError::TailSplitsToolPair
        );
    }
}
