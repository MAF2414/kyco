use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{LogEvent, ScopeDefinition};

/// Unique identifier for a job
pub type JobId = u64;

/// The status of a job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is waiting to be executed (not yet queued)
    Pending,
    /// Job is in the queue waiting to run
    Queued,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Done,
    /// Job failed during execution
    Failed,
    /// Job was rejected by the user
    Rejected,
    /// Job was merged into main branch
    Merged,
}

impl JobStatus {
    /// Get the status marker string used in comments
    pub fn as_marker(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
            JobStatus::Rejected => "rejected",
            JobStatus::Merged => "merged",
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_marker())
    }
}

/// A job represents the execution of a comment-based task by a coding agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier
    pub id: JobId,

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
}

impl Job {
    /// Create a new pending job
    pub fn new(
        id: JobId,
        mode: String,
        scope: ScopeDefinition,
        target: String,
        description: Option<String>,
        agent_id: String,
        source_file: PathBuf,
        source_line: usize,
        raw_tag_line: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            mode,
            scope,
            target,
            description,
            agent_id,
            status: JobStatus::Pending,
            created_at: now,
            updated_at: now,
            git_base_revision: None,
            git_worktree_path: None,
            branch_name: None,
            changed_files: Vec::new(),
            log_events: Vec::new(),
            error_message: None,
            source_file,
            source_line,
            raw_tag_line,
            sent_prompt: None,
        }
    }

    /// Update the job status
    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Add a log event
    pub fn add_log_event(&mut self, event: LogEvent) {
        self.log_events.push(event);
        self.updated_at = Utc::now();
    }

    /// Set the error message and mark as failed
    pub fn fail(&mut self, message: String) {
        self.error_message = Some(message);
        self.set_status(JobStatus::Failed);
    }

    /// Check if the job is in a terminal state
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Done | JobStatus::Failed | JobStatus::Rejected | JobStatus::Merged
        )
    }
}
