use chrono::Utc;
use std::path::PathBuf;

use super::{Job, JobId, JobResult, JobStats, JobStatus, MAX_JOB_LOG_EVENTS};
use crate::domain::{LogEvent, ScopeDefinition};

impl Job {
    /// Create a new pending job
    pub fn new(
        id: JobId,
        skill: String,
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
            workspace_path: None,
            skill,
            scope,
            target,
            description,
            agent_id,
            status: JobStatus::Pending,
            cancel_requested: false,
            cancel_sent: false,
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
            input_tokens: None,
            output_tokens: None,
            cache_read_tokens: None,
            cache_write_tokens: None,
            cost_usd: None,
            started_at: None,
            finished_at: None,
            group_id: None,
            ide_context: None,
            force_worktree: false,
            is_repl: false,
            bridge_session_id: None,
            fork_session: false,
            permission_mode: None,
            blocked_by: None,
            blocked_file: None,
            chain_step_history: Vec::new(),
            chain_current_step: None,
            chain_total_steps: None,
            chain_name: None,
            bugbounty_project_id: None,
            bugbounty_finding_ids: Vec::new(),
            structured_output: None,
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
        if self.log_events.len() > MAX_JOB_LOG_EVENTS {
            let excess = self.log_events.len() - MAX_JOB_LOG_EVENTS;
            self.log_events.drain(0..excess);
        }
        self.updated_at = Utc::now();
    }

    /// Set the error message and mark as failed
    pub fn fail(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
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
