//! Chain result types and data structures.

use std::sync::Arc;

use crate::JobResult;

/// Result of a single step in a chain.
///
/// Captures the outcome of executing one step, including whether it was skipped
/// due to trigger conditions not being met. When a step runs, both `job_result`
/// (parsed YAML output) and `agent_result` (execution metadata) are populated.
#[derive(Debug, Clone)]
pub struct ChainStepResult {
    /// The skill that was executed (e.g., "review", "fix").
    /// Uses `Arc<str>` for cheap cloning in repeated chain executions.
    pub skill: Arc<str>,
    /// Zero-based index of this step in the chain.
    pub step_index: usize,
    /// `true` if the step was skipped due to `trigger_on`/`skip_on` conditions.
    pub skipped: bool,
    /// Parsed job result from the agent's YAML output block, if the step ran.
    pub job_result: Option<JobResult>,
    /// Summary of agent execution (success, errors, file changes), if the step ran.
    pub agent_result: Option<AgentResultSummary>,
    /// Full response text from the agent (for UI display).
    pub full_response: Option<String>,
}

/// Summarized agent result for chain step tracking.
///
/// A lightweight view of [`super::AgentResult`] that omits large fields like
/// full output text and file diffs, suitable for storing in chain history.
#[derive(Debug, Clone)]
pub struct AgentResultSummary {
    /// Whether the agent completed without errors.
    pub success: bool,
    /// Error message if the agent failed.
    pub error: Option<String>,
    /// Number of files modified by this step.
    pub files_changed: usize,
}

/// Result of running a complete chain.
///
/// Contains the full execution history of the chain, including results from
/// each step (whether skipped or executed) and accumulated context summaries.
///
/// # Fields
///
/// - `success`: `true` only if all executed steps succeeded. A chain with
///   skipped steps can still be successful if no executed step failed.
/// - `final_state`: The state identifier from the last **executed** step,
///   used for downstream decision-making or reporting.
/// - `accumulated_summaries`: Ordered list of `"[mode] summary"` strings
///   from all executed steps, useful for debugging or generating reports.
#[derive(Debug)]
pub struct ChainResult {
    /// Name of the chain that was executed.
    pub chain_name: String,
    /// Results from each step in order, including skipped steps.
    pub step_results: Vec<ChainStepResult>,
    /// `true` if no executed step failed.
    pub success: bool,
    /// State identifier from the last executed step (e.g., "issues_found").
    pub final_state: Option<String>,
    /// Accumulated `"[mode] summary"` entries from all executed steps.
    pub accumulated_summaries: Vec<String>,
}

/// Progress event sent during chain execution for real-time UI updates.
#[derive(Debug, Clone)]
pub struct ChainProgressEvent {
    /// Current step index (0-based)
    pub step_index: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Skill being executed.
    /// Uses `Arc<str>` for cheap cloning in repeated chain executions.
    pub skill: Arc<str>,
    /// Whether the step is starting (true) or completed (false)
    pub is_starting: bool,
    /// Step result (only present when is_starting is false)
    pub step_result: Option<ChainStepResult>,
}
