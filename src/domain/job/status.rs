use serde::{Deserialize, Serialize};

/// The status of a job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is waiting to be executed (not yet queued)
    Pending,
    /// Job is in the queue waiting to run
    Queued,
    /// Job is blocked waiting for file lock (another job is editing the same file)
    Blocked,
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
            JobStatus::Blocked => "blocked",
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
