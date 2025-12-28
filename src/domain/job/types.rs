use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Computed statistics for a completed job
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobStats {
    /// Number of files changed
    pub files_changed: usize,
    /// Lines added
    pub lines_added: usize,
    /// Lines removed
    pub lines_removed: usize,
    /// Duration of the job
    pub duration: Option<Duration>,
}

/// Summary of a completed chain step (for UI display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStepSummary {
    /// Step index (0-based)
    pub step_index: usize,
    /// Mode that was executed
    pub mode: String,
    /// Whether the step was skipped due to trigger conditions
    pub skipped: bool,
    /// Whether the step succeeded
    pub success: bool,
    /// Short title from the step result
    pub title: Option<String>,
    /// Summary text (for context passing display)
    pub summary: Option<String>,
    /// Full response text from the agent
    pub full_response: Option<String>,
    /// Error message if the step failed
    pub error: Option<String>,
    /// Number of files changed by this step
    pub files_changed: usize,
}
