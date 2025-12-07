//! Agent run group for parallel multi-agent execution
//!
//! An AgentRunGroup represents a collection of jobs that all process the same
//! prompt in parallel, but with different agents. This enables users to compare
//! results from multiple agents (e.g., claude+codex+gemini) and select the best one.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::JobId;

/// Unique identifier for an agent run group
pub type AgentGroupId = u64;

/// Status of an agent run group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupStatus {
    /// At least one job is still running
    Running,
    /// All jobs finished, waiting for user to select the best result
    Comparing,
    /// User has selected a result
    Selected,
    /// Selected result has been merged into main branch
    Merged,
    /// Group was cancelled or all jobs failed
    Cancelled,
}

impl GroupStatus {
    /// Get the status as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupStatus::Running => "running",
            GroupStatus::Comparing => "comparing",
            GroupStatus::Selected => "selected",
            GroupStatus::Merged => "merged",
            GroupStatus::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for GroupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A group of jobs that process the same prompt with different agents
///
/// This enables the multi-agent workflow where users can:
/// 1. Send the same task to multiple agents in parallel
/// 2. Compare the results side-by-side
/// 3. Select and merge the best result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunGroup {
    /// Unique identifier for this group
    pub id: AgentGroupId,

    /// The prompt/description shared by all jobs in this group
    pub prompt: String,

    /// The mode used for all jobs (e.g., "refactor", "fix")
    pub mode: String,

    /// The target being processed (e.g., "src/lib.rs:42")
    pub target: String,

    /// IDs of all jobs in this group
    pub job_ids: Vec<JobId>,

    /// Names of agents in this group (in same order as job_ids)
    pub agent_names: Vec<String>,

    /// Current status of the group
    pub status: GroupStatus,

    /// The job selected by the user (if status is Selected or Merged)
    pub selected_job: Option<JobId>,

    /// When this group was created
    pub created_at: DateTime<Utc>,

    /// When this group was last updated
    pub updated_at: DateTime<Utc>,
}

impl AgentRunGroup {
    /// Create a new agent run group
    pub fn new(
        id: AgentGroupId,
        prompt: String,
        mode: String,
        target: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            prompt,
            mode,
            target,
            job_ids: Vec::new(),
            agent_names: Vec::new(),
            status: GroupStatus::Running,
            selected_job: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a job to this group
    pub fn add_job(&mut self, job_id: JobId, agent_name: String) {
        self.job_ids.push(job_id);
        self.agent_names.push(agent_name);
        self.updated_at = Utc::now();
    }

    /// Set the group status
    pub fn set_status(&mut self, status: GroupStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Select a job as the winning result
    pub fn select_job(&mut self, job_id: JobId) {
        self.selected_job = Some(job_id);
        self.status = GroupStatus::Selected;
        self.updated_at = Utc::now();
    }

    /// Mark the group as merged
    pub fn mark_merged(&mut self) {
        self.status = GroupStatus::Merged;
        self.updated_at = Utc::now();
    }

    /// Get the number of jobs in this group
    pub fn job_count(&self) -> usize {
        self.job_ids.len()
    }

    /// Check if all jobs in the group are finished
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            GroupStatus::Comparing | GroupStatus::Selected | GroupStatus::Merged | GroupStatus::Cancelled
        )
    }

    /// Get the agent name for a given job ID
    pub fn agent_for_job(&self, job_id: JobId) -> Option<&str> {
        self.job_ids
            .iter()
            .position(|&id| id == job_id)
            .and_then(|idx| self.agent_names.get(idx))
            .map(|s| s.as_str())
    }
}
