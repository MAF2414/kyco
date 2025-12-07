use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use super::{AgentGroupId, LogEvent, ScopeDefinition};

/// Parsed output from the agent's ---kyco block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobResult {
    /// Short title describing what was done
    pub title: Option<String>,
    /// Detailed description (2-3 sentences)
    pub details: Option<String>,
    /// Status: success, partial, or failed
    pub status: Option<String>,
    /// Longer summary for chain context (can be multiline, passed to next agent)
    pub summary: Option<String>,
    /// State identifier for chain triggers (e.g., "issues_found", "fixed", "tests_pass")
    pub state: Option<String>,
    /// Structured context data for next agent in chain
    pub next_context: Option<serde_json::Value>,
}

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

impl JobResult {
    /// Parse a ---kyco YAML block from agent output
    ///
    /// Supports both simple key: value pairs and multiline values using YAML block syntax:
    /// ```yaml
    /// ---kyco
    /// title: Short title
    /// summary: |
    ///   This is a multiline summary
    ///   that spans multiple lines.
    /// state: issues_found
    /// ---
    /// ```
    pub fn parse(output: &str) -> Option<Self> {
        // Find the ---kyco block
        let start_marker = "---kyco";
        let end_marker = "---";

        let start_idx = output.find(start_marker)?;
        let content_start = start_idx + start_marker.len();

        // Find the closing --- after the opening ---kyco
        let remaining = &output[content_start..];
        let end_idx = remaining.find(end_marker)?;

        let yaml_content = remaining[..end_idx].trim();

        // Try to parse as proper YAML first (handles multiline values)
        if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
            if let serde_yaml::Value::Mapping(map) = yaml_value {
                let mut result = JobResult::default();

                for (key, value) in map {
                    if let serde_yaml::Value::String(key_str) = key {
                        match key_str.as_str() {
                            "title" => result.title = value_to_string(&value),
                            "details" => result.details = value_to_string(&value),
                            "status" => result.status = value_to_string(&value),
                            "summary" => result.summary = value_to_string(&value),
                            "state" => result.state = value_to_string(&value),
                            "next_context" => {
                                result.next_context = yaml_to_json(&value);
                            }
                            _ => {}
                        }
                    }
                }

                if result.title.is_some() || result.status.is_some() {
                    return Some(result);
                }
            }
        }

        // Fallback: Parse simple key: value pairs (backwards compatibility)
        let mut result = JobResult::default();

        for line in yaml_content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "title" => result.title = Some(value.to_string()),
                    "details" => result.details = Some(value.to_string()),
                    "status" => result.status = Some(value.to_string()),
                    "summary" => result.summary = Some(value.to_string()),
                    "state" => result.state = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Only return if we got at least a title or status
        if result.title.is_some() || result.status.is_some() {
            Some(result)
        } else {
            None
        }
    }
}

/// Convert a YAML value to an optional String
fn value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Convert a YAML value to a JSON value (for next_context)
fn yaml_to_json(value: &serde_yaml::Value) -> Option<serde_json::Value> {
    match value {
        serde_yaml::Value::Null => Some(serde_json::Value::Null),
        serde_yaml::Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(serde_json::Value::Number(i.into()))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f).map(serde_json::Value::Number)
            } else {
                None
            }
        }
        serde_yaml::Value::String(s) => Some(serde_json::Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let json_arr: Option<Vec<_>> = seq.iter().map(yaml_to_json).collect();
            json_arr.map(serde_json::Value::Array)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut json_obj = serde_json::Map::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    if let Some(json_v) = yaml_to_json(v) {
                        json_obj.insert(key.clone(), json_v);
                    }
                }
            }
            Some(serde_json::Value::Object(json_obj))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

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

    /// Parsed result from the agent's ---kyco output block
    #[serde(default)]
    pub result: Option<JobResult>,

    /// Computed statistics (files changed, lines, duration)
    #[serde(default)]
    pub stats: Option<JobStats>,

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
            result: None,
            stats: None,
            started_at: None,
            finished_at: None,
            group_id: None,
            ide_context: None,
            force_worktree: false,
        }
    }

    /// Update the job status
    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        // Track timing
        if status == JobStatus::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }

        if matches!(status, JobStatus::Done | JobStatus::Failed | JobStatus::Rejected) {
            self.finished_at = Some(Utc::now());
            self.compute_duration();
        }
    }

    /// Compute duration from started_at to finished_at
    fn compute_duration(&mut self) {
        if let (Some(start), Some(end)) = (self.started_at, self.finished_at) {
            let duration = end.signed_duration_since(start);
            if let Ok(std_duration) = duration.to_std() {
                if let Some(stats) = &mut self.stats {
                    stats.duration = Some(std_duration);
                } else {
                    self.stats = Some(JobStats {
                        duration: Some(std_duration),
                        ..Default::default()
                    });
                }
            }
        }
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

    /// Parse agent output and extract the ---kyco result block
    pub fn parse_result(&mut self, output: &str) {
        self.result = JobResult::parse(output);
    }

    /// Update stats with file change information
    pub fn set_file_stats(&mut self, files_changed: usize, lines_added: usize, lines_removed: usize) {
        if let Some(stats) = &mut self.stats {
            stats.files_changed = files_changed;
            stats.lines_added = lines_added;
            stats.lines_removed = lines_removed;
        } else {
            self.stats = Some(JobStats {
                files_changed,
                lines_added,
                lines_removed,
                duration: None,
            });
        }
    }

    /// Get a formatted duration string (e.g., "1m 23s", "45s")
    pub fn duration_string(&self) -> Option<String> {
        let duration = self.stats.as_ref()?.duration?;
        let secs = duration.as_secs();

        if secs >= 60 {
            Some(format!("{}m {}s", secs / 60, secs % 60))
        } else {
            Some(format!("{}s", secs))
        }
    }
}
