use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{JobId, JobStatus, Scope, Target};

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
/// Examples:
/// - @claude#refactor.block.function
/// - @c#r.f (short form)
/// - @codex#tests.all.file add integration tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentTag {
    /// The file where this comment was found
    pub file_path: PathBuf,

    /// The line number (1-indexed)
    pub line_number: usize,

    /// The raw comment line
    pub raw_line: String,

    /// The agent to use (e.g., "claude", "codex", "gemini")
    pub agent: String,

    /// The parsed mode (e.g., "refactor", "tests", "docs")
    pub mode: String,

    /// The target - what to process (block, all, comments, etc.)
    pub target: Target,

    /// The parsed scope
    pub scope: Scope,

    /// Optional status marker if present
    pub status_marker: Option<StatusMarker>,

    /// Description text (inline or from following comment lines)
    pub description: Option<String>,

    /// The associated job ID (if linked to a job)
    pub job_id: Option<JobId>,
}

impl CommentTag {
    /// Create a new comment tag with the new simplified syntax (no target/scope)
    pub fn new_simple(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        agent: String,
        mode: String,
    ) -> Self {
        Self {
            file_path,
            line_number,
            raw_line,
            agent,
            mode,
            target: Target::Block,  // Legacy default
            scope: Scope::Function, // Legacy default
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Create a new comment tag with the full syntax (legacy)
    pub fn new(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        agent: String,
        mode: String,
        target: Target,
        scope: Scope,
    ) -> Self {
        Self {
            file_path,
            line_number,
            raw_line,
            agent,
            mode,
            target,
            scope,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Create a comment tag with defaults (for legacy compatibility)
    pub fn with_defaults(
        file_path: PathBuf,
        line_number: usize,
        raw_line: String,
        mode: String,
        scope: Scope,
    ) -> Self {
        Self {
            file_path,
            line_number,
            raw_line,
            agent: "claude".to_string(),
            mode,
            target: Target::Block,
            scope,
            status_marker: None,
            description: None,
            job_id: None,
        }
    }

    /// Check if this comment is already linked to a job
    pub fn is_linked(&self) -> bool {
        self.job_id.is_some() || self.status_marker.is_some()
    }

    /// Get the job ID from either the explicit field or the status marker
    pub fn get_job_id(&self) -> Option<JobId> {
        self.job_id.or(self.status_marker.as_ref().map(|m| m.job_id))
    }

    /// Generate the updated comment line with a status marker (new syntax)
    pub fn with_status(&self, status: JobStatus, job_id: JobId) -> String {
        let marker = StatusMarker { status, job_id };
        // Use canonical (short) form: @agent#mode.target.scope [status#id]
        format!(
            "// @{}#{}.{}.{} {}",
            self.agent,
            self.mode,
            self.target.short(),
            self.scope.short(),
            marker
        )
    }

    /// Generate the marker string without status
    pub fn to_marker_string(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("");
        if desc.is_empty() {
            format!(
                "@{}#{}.{}.{}",
                self.agent,
                self.mode,
                self.target.short(),
                self.scope.short()
            )
        } else {
            format!(
                "@{}#{}.{}.{} {}",
                self.agent,
                self.mode,
                self.target.short(),
                self.scope.short(),
                desc
            )
        }
    }
}
