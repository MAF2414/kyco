//! Target type definitions for KYCo markers

use serde::{Deserialize, Serialize};

/// The target of a KYCo marker - what code should be processed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// Only process marked comments (@cr: markers within the scope)
    Comments,
    /// The next code block after the marker
    #[default]
    Block,
    /// Editor selection (for IDE integration)
    Selection,
    /// Everything within the scope
    All,
    /// Explicitly marked regions (@cr:start ... @cr:end)
    Marked,
}

impl Target {
    /// Parse a target from a string (supports short aliases)
    /// - comments: c, com, comments, comment
    /// - block: b, blk, block, next
    /// - selection: s, sel, selection
    /// - all: a, all, full
    /// - marked: m, mark, marked, region
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "c" | "com" | "comments" | "comment" => Some(Target::Comments),
            "b" | "blk" | "block" | "next" => Some(Target::Block),
            "s" | "sel" | "selection" => Some(Target::Selection),
            "a" | "all" | "full" => Some(Target::All),
            "m" | "mark" | "marked" | "region" => Some(Target::Marked),
            _ => None,
        }
    }

    /// Get the canonical string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Target::Comments => "comments",
            Target::Block => "block",
            Target::Selection => "selection",
            Target::All => "all",
            Target::Marked => "marked",
        }
    }

    /// Get the short alias
    pub fn short(&self) -> &'static str {
        match self {
            Target::Comments => "c",
            Target::Block => "b",
            Target::Selection => "s",
            Target::All => "a",
            Target::Marked => "m",
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
