use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{JobId, JobStatus, Target};

/// A status marker found in a comment (e.g., [pending#42])
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMarker {
    /// The status indicated
    pub status: JobStatus,
    /// The job ID
    pub job_id: JobId,
}

impl StatusMarker {
    pub fn parse(s: &str) -> Option<Self> {
        // Use split_once to avoid Vec allocation
        let (status_str, job_id_str) = s.split_once('#')?;

        let status = match status_str {
            "pending" => JobStatus::Pending,
            "queued" => JobStatus::Queued,
            "running" => JobStatus::Running,
            "done" => JobStatus::Done,
            "failed" => JobStatus::Failed,
            "rejected" => JobStatus::Rejected,
            _ => return None,
        };

        let job_id: JobId = job_id_str.parse().ok()?;

        Some(Self { status, job_id })
    }

    /// Format as a marker string
    pub fn to_marker_string(&self) -> String {
        format!("[{}#{}]", self.status.as_marker(), self.job_id)
    }
}

impl std::fmt::Display for StatusMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_marker_string())
    }
}

/// A parsed comment tag from the source code
///
/// Supports the new syntax: @agent#mode.target.scope description
/// Multi-agent syntax: @agent1+agent2+agent3:mode description
///
/// Examples:
/// - @claude#refactor.block.function
/// - @c#r.f (short form)
/// - @codex#tests.all.file add integration tests
/// - @claude+codex+gemini:refactor optimize this function (parallel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentTag {
    /// The file where this comment was found
    pub file_path: PathBuf,

    /// The line number (1-indexed)
    pub line_number: usize,

    /// The raw comment line
    pub raw_line: String,

    /// The agent to use (e.g., "claude", "codex", "gemini")
    /// For single-agent tags, this is the agent name.
    /// For multi-agent tags, this is the first agent (for backwards compatibility).
    pub agent: String,

    /// Multiple agents for parallel execution (e.g., ["claude", "codex", "gemini"])
    /// If this has more than one agent, the same task will be run in parallel.
    #[serde(default)]
    pub agents: Vec<String>,

    /// The parsed mode (e.g., "refactor", "tests", "docs")
    pub mode: String,

    /// The target - what to process (block, all, comments, etc.)
    pub target: Target,

    /// Optional status marker if present
    pub status_marker: Option<StatusMarker>,

    /// Description text (inline or from following comment lines)
    pub description: Option<String>,

    /// The associated job ID (if linked to a job)
    pub job_id: Option<JobId>,
}

impl CommentTag {
    /// Create a new comment tag with the simplified syntax
    pub fn new_simple(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        agent: String,
        mode: String,
    ) -> Self {
        let agents = vec![agent.clone()];
        Self {
            file_path,
            line_number,
            raw_line,
            agent,
            agents,
            mode,
            target: Target::Block,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Create a new comment tag with target
    pub fn new(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        agent: String,
        mode: String,
        target: Target,
    ) -> Self {
        let agents = vec![agent.clone()];
        Self {
            file_path,
            line_number,
            raw_line,
            agent,
            agents,
            mode,
            target,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Create a new comment tag with multiple agents for parallel execution
    pub fn new_multi_agent(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        agents: Vec<String>,
        mode: String,
        target: Target,
    ) -> Self {
        let agent = agents.first().cloned().unwrap_or_else(|| "claude".to_string());
        Self {
            file_path,
            line_number,
            raw_line,
            agent,
            agents,
            mode,
            target,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Create a comment tag with defaults
    pub fn with_defaults(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        mode: String,
    ) -> Self {
        Self {
            file_path,
            line_number,
            raw_line,
            agent: "claude".to_string(),
            agents: vec!["claude".to_string()],
            mode,
            target: Target::Block,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Check if this tag has multiple agents (parallel execution)
    pub fn is_multi_agent(&self) -> bool {
        self.agents.len() > 1
    }

    /// Check if this comment is already linked to a job
    pub fn is_linked(&self) -> bool {
        self.job_id.is_some() || self.status_marker.is_some()
    }

    /// Get the job ID from either the explicit field or the status marker
    pub fn get_job_id(&self) -> Option<JobId> {
        self.job_id.or(self.status_marker.as_ref().map(|m| m.job_id))
    }

    /// Generate the updated comment line with a status marker
    pub fn with_status(&self, status: JobStatus, job_id: JobId) -> String {
        let marker = StatusMarker { status, job_id };
        format!(
            "// @{}:{} {}",
            self.agent,
            self.mode,
            marker
        )
    }

    /// Generate the marker string without status
    pub fn to_marker_string(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("");
        if desc.is_empty() {
            format!(
                "@{}:{}",
                self.agent,
                self.mode,
            )
        } else {
            format!(
                "@{}:{} {}",
                self.agent,
                self.mode,
                desc
            )
        }
    }
}
