mod impls;
mod parse;
mod result;
mod status;
mod types;

pub use result::JobResult;
pub use status::JobStatus;
pub use types::{ChainStepSummary, JobStats};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{AgentGroupId, LogEvent, ScopeDefinition};

/// Maximum number of log events to keep per job (FIFO eviction)
/// Prevents unbounded memory growth from tool call accumulation
pub(super) const MAX_JOB_LOG_EVENTS: usize = 200;

/// Unique identifier for a job
pub type JobId = u64;

/// A job represents the execution of a comment-based task by a coding agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier
    pub id: JobId,

    /// Workspace root path (for SDK cwd resolution)
    #[serde(default)]
    pub workspace_path: Option<PathBuf>,

    /// The mode of the job (e.g., "refactor", "tests", "docs", "review")
    pub mode: String,

    /// The scope definition for this job
    pub scope: ScopeDefinition,

    /// Human-readable target description (e.g., "process_order in src/orders.rs")
    pub target: String,

    /// Description text from the comment (second line onwards)
    pub description: Option<String>,

    /// The agent to use for this job (e.g., "claude")
    pub agent_id: String,

    /// Current status of the job
    pub status: JobStatus,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// When the job was last updated
    pub updated_at: DateTime<Utc>,

    /// The Git commit SHA when the job was created
    pub git_base_revision: Option<String>,

    /// Path to the Git worktree for this job
    pub git_worktree_path: Option<PathBuf>,

    /// Branch name for this job's worktree
    pub branch_name: Option<String>,

    /// The base branch from which the worktree was created (for merging back)
    pub base_branch: Option<String>,

    /// Files changed by this job (populated after execution)
    pub changed_files: Vec<PathBuf>,

    /// Log events from the agent execution
    pub log_events: Vec<LogEvent>,

    /// Error message if the job failed
    pub error_message: Option<String>,

    /// The source file where the comment was found
    pub source_file: PathBuf,

    /// The line number of the comment in the source file
    pub source_line: usize,

    /// The raw comment line as found by the scanner (for removal before agent runs)
    #[serde(default)]
    pub raw_tag_line: Option<String>,

    /// The full prompt sent to the model (set when job starts running)
    #[serde(default)]
    pub sent_prompt: Option<String>,

    /// The full raw text response from the agent (for display)
    #[serde(default)]
    pub full_response: Option<String>,

    /// Parsed result from the agent's ---kyco output block
    #[serde(default)]
    pub result: Option<JobResult>,

    /// Computed statistics (files changed, lines, duration)
    #[serde(default)]
    pub stats: Option<JobStats>,

    /// Input tokens used (from API response)
    #[serde(default)]
    pub input_tokens: Option<u64>,

    /// Output tokens generated (from API response)
    #[serde(default)]
    pub output_tokens: Option<u64>,

    /// Cache read tokens (prompt caching)
    #[serde(default)]
    pub cache_read_tokens: Option<u64>,

    /// Cache write tokens (prompt caching)
    #[serde(default)]
    pub cache_write_tokens: Option<u64>,

    /// Total cost in USD (from API response)
    #[serde(default)]
    pub cost_usd: Option<f64>,

    /// When the job started running
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,

    /// When the job finished (done/failed)
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,

    /// ID of the agent run group this job belongs to (for parallel multi-agent execution)
    #[serde(default)]
    pub group_id: Option<AgentGroupId>,

    /// IDE context markdown (dependencies, related tests) for prompt injection
    #[serde(default)]
    pub ide_context: Option<String>,

    /// Force this job to run in a git worktree, regardless of global settings
    #[serde(default)]
    pub force_worktree: bool,

    /// Legacy: Whether this job ran in Terminal REPL mode
    #[serde(default)]
    pub is_repl: bool,

    /// Bridge session ID for session continuation
    /// Allows sending follow-up prompts to continue the conversation
    #[serde(default)]
    pub bridge_session_id: Option<String>,

    /// Job ID that is blocking this job (when status is Blocked)
    /// This happens when another job holds a file lock on the same file
    #[serde(default)]
    pub blocked_by: Option<JobId>,

    /// The file path that is causing the block
    #[serde(default)]
    pub blocked_file: Option<PathBuf>,

    /// Chain step history (for chain jobs - shows progress and intermediate results)
    #[serde(default)]
    pub chain_step_history: Vec<ChainStepSummary>,

    /// Current chain step index (0-based, None if not a chain job or not started)
    #[serde(default)]
    pub chain_current_step: Option<usize>,

    /// Total number of steps in the chain (None if not a chain job)
    #[serde(default)]
    pub chain_total_steps: Option<usize>,

    /// Name of the chain being executed (None if not a chain job)
    #[serde(default)]
    pub chain_name: Option<String>,
}
