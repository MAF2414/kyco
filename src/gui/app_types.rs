//! Type definitions for KycoApp
//!
//! Contains ViewMode and Mode enums extracted from app.rs.

/// View mode for the app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main job list view
    JobList,
    /// Selection popup (triggered by IDE extension)
    SelectionPopup,
    /// Batch selection popup (triggered by IDE batch request)
    BatchPopup,
    /// Diff view popup
    DiffView,
    /// Apply/merge confirmation popup
    ApplyConfirmPopup,
    /// Comparison popup for multi-agent results
    ComparisonPopup,
    /// Settings/Extensions view
    Settings,
    /// Modes configuration view
    Modes,
    /// Agents configuration view
    Agents,
    /// Chains configuration view
    Chains,
    /// Statistics dashboard view
    Stats,
}

// Keep old types for compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Refactor,
    Fix,
    Tests,
    Docs,
    Review,
    Optimize,
    Implement,
    Custom,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Refactor => "refactor",
            Mode::Fix => "fix",
            Mode::Tests => "tests",
            Mode::Docs => "docs",
            Mode::Review => "review",
            Mode::Optimize => "optimize",
            Mode::Implement => "implement",
            Mode::Custom => "custom",
        }
    }
}
