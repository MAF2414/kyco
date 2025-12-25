use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;

use super::{AgentGroupId, LogEvent, ScopeDefinition};
use crate::workspace::WorkspaceId;

/// Maximum number of log events to keep per job (FIFO eviction)
/// Prevents unbounded memory growth from tool call accumulation
const MAX_JOB_LOG_EVENTS: usize = 200;

/// Parsed output from the agent's YAML summary block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobResult {
    /// Short title describing what was done
    pub title: Option<String>,
    /// Suggested git commit subject line
    pub commit_subject: Option<String>,
    /// Suggested git commit body
    pub commit_body: Option<String>,
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
    /// Raw text output when no structured YAML is found
    pub raw_text: Option<String>,
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

impl JobResult {
    /// Parse a YAML summary block from agent output
    ///
    /// Supports multiple formats:
    /// 1. Standard YAML front matter with `---` markers
    /// 2. Legacy `---kyco` markers (backwards compatibility)
    /// 3. Falls back to raw text if no YAML structure found
    ///
    /// ```yaml
    /// ---
    /// title: Short title
    /// summary: |
    ///   This is a multiline summary
    ///   that spans multiple lines.
    /// state: issues_found
    /// ---
    /// ```
    pub fn parse(output: &str) -> Option<Self> {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Some runners return a JSON string literal as the "result" payload.
        // If we don't unwrap it here, the UI ends up showing quotes and escaped newlines (\"...\\n...\")
        // and YAML parsing fails because keys become "\\ntitle", "\\nstatus", etc.
        let output: Cow<'_, str> = unwrap_json_string_literal(trimmed)
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(trimmed));
        let output = output.as_ref().trim();

        // Try standard YAML markers first, then legacy ---kyco
        if let Some(result) = Self::parse_yaml_block(output, "---") {
            return Some(result);
        }
        if let Some(result) = Self::parse_yaml_block(output, "---kyco") {
            return Some(result);
        }

        // Try JSON structured output (SDK outputFormat / outputSchema)
        if let Some(result) = Self::parse_json_block(output) {
            return Some(result);
        }

        // No structured YAML found - extract raw text from the output
        // For SDK output, we get the assistant's text directly
        // Trim and take meaningful content
        if !output.is_empty() {
            return Some(JobResult {
                raw_text: Some(output.to_string()),
                ..Default::default()
            });
        }

        None
    }

    fn parse_json_block(output: &str) -> Option<Self> {
        let raw = output.trim();
        if !raw.starts_with('{') {
            return None;
        }

        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let obj = value.as_object()?;

        let mut result = JobResult::default();
        result.title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.commit_subject = obj
            .get("commit_subject")
            .or_else(|| obj.get("commitSubject"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.commit_body = obj
            .get("commit_body")
            .or_else(|| obj.get("commitBody"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.details = obj
            .get("details")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.state = obj
            .get("state")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        result.next_context = obj
            .get("next_context")
            .cloned()
            .or_else(|| obj.get("nextContext").cloned());

        result.summary = obj.get("summary").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            other => serde_json::to_string_pretty(other).ok(),
        });

        let has_structured = result.title.is_some()
            || result.commit_subject.is_some()
            || result.commit_body.is_some()
            || result.details.is_some()
            || result.status.is_some()
            || result.summary.is_some()
            || result.state.is_some()
            || result.next_context.is_some();

        if has_structured { Some(result) } else { None }
    }

    /// Parse a YAML block with a specific start marker
    fn parse_yaml_block(output: &str, start_marker: &str) -> Option<Self> {
        let end_marker = "---";

        // Find the start marker
        let start_idx = output.find(start_marker)?;
        let content_start = start_idx + start_marker.len();

        // Find the closing --- after the start marker
        let remaining = &output[content_start..];

        // For standard `---`, we need to find the NEXT `---` (not the same one)
        // For `---kyco`, the next `---` is always the closing one
        let end_idx = if start_marker == "---" {
            // Skip whitespace and find next ---
            remaining.trim_start().find(end_marker).map(|i| {
                // Adjust for trimmed whitespace
                remaining.len() - remaining.trim_start().len() + i
            })?
        } else {
            remaining.find(end_marker)?
        };

        // For standard ---, we need to handle the case where content is between the two markers
        let yaml_content = if start_marker == "---" {
            // The content is what's after the first --- until the next ---
            remaining[..end_idx].trim()
        } else {
            remaining[..end_idx].trim()
        };

        // Skip if content is empty or too short
        if yaml_content.is_empty() || yaml_content.len() < 5 {
            return None;
        }

        // Try to parse as proper YAML first (handles multiline values)
        if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
            if let serde_yaml::Value::Mapping(map) = yaml_value {
                let mut result = JobResult::default();

                for (key, value) in map {
                    if let serde_yaml::Value::String(key_str) = key {
                        match key_str.as_str() {
                            "title" => result.title = value_to_string(&value),
                            "commit_subject" => result.commit_subject = value_to_string(&value),
                            "commit_body" => result.commit_body = value_to_string(&value),
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

                if result.title.is_some()
                    || result.status.is_some()
                    || result.commit_subject.is_some()
                    || result.commit_body.is_some()
                {
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
                    "commit_subject" => result.commit_subject = Some(value.to_string()),
                    "commit_body" => result.commit_body = Some(value.to_string()),
                    "details" => result.details = Some(value.to_string()),
                    "status" => result.status = Some(value.to_string()),
                    "summary" => result.summary = Some(value.to_string()),
                    "state" => result.state = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Only return if we got at least a title or status
        if result.title.is_some()
            || result.status.is_some()
            || result.commit_subject.is_some()
            || result.commit_body.is_some()
        {
            Some(result)
        } else {
            None
        }
    }
}

fn unwrap_json_string_literal(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if !(trimmed.starts_with('"') && trimmed.ends_with('"')) {
        return None;
    }

    serde_json::from_str::<String>(trimmed).ok()
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

/// A job represents the execution of a comment-based task by a coding agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier
    pub id: JobId,

    /// Workspace this job belongs to (for multi-workspace support)
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,

    /// Workspace root path (for display and cwd resolution)
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
            workspace_id: None,
            workspace_path: None,
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
            base_branch: None,
            changed_files: Vec::new(),
            log_events: Vec::new(),
            error_message: None,
            source_file,
            source_line,
            raw_tag_line,
            sent_prompt: None,
            full_response: None,
            result: None,
            stats: None,
            started_at: None,
            finished_at: None,
            group_id: None,
            ide_context: None,
            force_worktree: false,
            is_repl: false,
            bridge_session_id: None,
            blocked_by: None,
            blocked_file: None,
            chain_step_history: Vec::new(),
            chain_current_step: None,
            chain_total_steps: None,
            chain_name: None,
        }
    }

    /// Create a new pending job with workspace association
    pub fn new_with_workspace(
        id: JobId,
        workspace_id: WorkspaceId,
        workspace_path: PathBuf,
        mode: String,
        scope: ScopeDefinition,
        target: String,
        description: Option<String>,
        agent_id: String,
        source_file: PathBuf,
        source_line: usize,
        raw_tag_line: Option<String>,
    ) -> Self {
        let mut job = Self::new(
            id,
            mode,
            scope,
            target,
            description,
            agent_id,
            source_file,
            source_line,
            raw_tag_line,
        );
        job.workspace_id = Some(workspace_id);
        job.workspace_path = Some(workspace_path);
        job
    }

    /// Update the job status
    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        // Track timing
        if status == JobStatus::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }

        if matches!(
            status,
            JobStatus::Done | JobStatus::Failed | JobStatus::Rejected
        ) {
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

    /// Add a log event, automatically removing oldest entries if over limit.
    /// This prevents unbounded memory growth from tool call accumulation.
    pub fn add_log_event(&mut self, event: LogEvent) {
        self.log_events.push(event);
        // Remove oldest entries if over limit (FIFO eviction)
        if self.log_events.len() > MAX_JOB_LOG_EVENTS {
            let excess = self.log_events.len() - MAX_JOB_LOG_EVENTS;
            self.log_events.drain(0..excess);
        }
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
    pub fn set_file_stats(
        &mut self,
        files_changed: usize,
        lines_added: usize,
        lines_removed: usize,
    ) {
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

#[cfg(test)]
mod tests {
    use super::JobResult;

    #[test]
    fn parse_unwraps_json_string_literal_then_parses_yaml_block() {
        let inner = r#"I need permission to read the file.

---
title: Code review blocked - permission needed
commit_subject: N/A
details: Unable to read the requested file due to permission restrictions.
status: blocked
summary: |
  Cannot proceed with code review - file read permission was denied.
state: blocked
---
"#;

        let wrapped = serde_json::to_string(inner).expect("json wrap");
        let result = JobResult::parse(&wrapped).expect("parse");

        assert_eq!(
            result.title.as_deref(),
            Some("Code review blocked - permission needed")
        );
        assert_eq!(result.status.as_deref(), Some("blocked"));
        assert!(result.raw_text.is_none());
    }

    #[test]
    fn parse_unwraps_json_string_literal_for_raw_text() {
        let inner = "hello\nworld";
        let wrapped = serde_json::to_string(inner).expect("json wrap");
        let result = JobResult::parse(&wrapped).expect("parse");

        assert_eq!(result.raw_text.as_deref(), Some("hello\nworld"));
    }
}
