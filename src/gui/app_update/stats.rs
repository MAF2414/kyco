//! Statistics recording for completed jobs

use crate::config::AgentConfigToml;
use crate::gui::app::KycoApp;
use crate::stats::{FileAccessType, FileStatsRecord, JobStatsRecord, ToolStatsRecord};
use crate::{Job, LogEvent};

impl KycoApp {
    /// Record job statistics when a job completes (success or failure)
    pub(crate) fn record_job_stats(&mut self, job_id: u64) {
        let Some(stats_manager) = &self.stats_manager else {
            return;
        };

        // Get job data from manager
        let job_data = if let Ok(manager) = self.job_manager.lock() {
            manager.get(job_id).cloned()
        } else {
            None
        };

        let Some(job) = job_data else {
            return;
        };

        // Get agent config for pricing (if available)
        let agent_config = self
            .config
            .read()
            .ok()
            .and_then(|cfg| cfg.agent.get(&job.agent_id).cloned());

        let record = job_to_stats_record(&job, agent_config.as_ref());
        if let Err(e) = stats_manager.recorder().record_job(&record) {
            tracing::warn!("Failed to record job stats: {}", e);
        }
    }

    /// Record a tool call from a LogEvent
    pub(crate) fn record_tool_call_from_event(&mut self, event: &LogEvent) {
        let Some(stats_manager) = &self.stats_manager else {
            return;
        };

        let Some(job_id) = event.job_id else {
            return;
        };

        let tool_name = event.tool_name.clone().unwrap_or_else(|| "unknown".to_string());
        let tool_use_id = event
            .tool_args
            .as_ref()
            .and_then(|args| args.get("tool_use_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Get session_id from job if available
        let session_id = if let Ok(manager) = self.job_manager.lock() {
            manager.get(job_id).and_then(|j| j.bridge_session_id.clone())
        } else {
            None
        };

        let timestamp = chrono::Utc::now().timestamp_millis();

        // Record tool call
        let record = ToolStatsRecord {
            job_id,
            session_id: session_id.clone(),
            tool_name: tool_name.clone(),
            tool_use_id,
            success: true, // We record on tool use, success is determined later
            timestamp,
        };

        if let Err(e) = stats_manager.recorder().record_tool_call(&record) {
            tracing::warn!("Failed to record tool call stats: {}", e);
        }

        // Also record file access if we can find a file path in the tool args
        if let Some(file_path) = extract_file_path_from_args(event.tool_args.as_ref()) {
            // Normalize worktree paths back to original paths
            let normalized_path = normalize_worktree_path(&file_path);

            // Skip if path is empty (e.g., worktree directory without file)
            if normalized_path.is_empty() {
                return;
            }

            let access_type = match tool_name.as_str() {
                "Write" | "NotebookEdit" => FileAccessType::Write,
                "Edit" => FileAccessType::Edit,
                _ => FileAccessType::Read, // Default to read for Glob, Grep, Read, LSP, etc.
            };

            let file_record = FileStatsRecord {
                job_id,
                session_id,
                file_path: normalized_path,
                access_type,
                timestamp,
            };

            if let Err(e) = stats_manager.recorder().record_file_access(&file_record) {
                tracing::warn!("Failed to record file access stats: {}", e);
            }
        }
    }
}

/// Extract file path from any tool arguments by checking common parameter names
fn extract_file_path_from_args(args: Option<&serde_json::Value>) -> Option<String> {
    let args = args?;

    // Common parameter names for file paths across different tools
    const PATH_PARAMS: &[&str] = &[
        "file_path",    // Read, Write, Edit
        "filePath",     // LSP
        "path",         // Glob, Grep
        "notebook_path", // NotebookEdit
    ];

    for param in PATH_PARAMS {
        if let Some(path) = args.get(*param).and_then(|v| v.as_str()) {
            // Skip if it's just a directory or pattern
            if !path.is_empty() && !path.ends_with('/') && !path.contains('*') {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Normalize worktree paths back to original repository paths
/// e.g., ".kyco/worktrees/job-5/src/main.rs" -> "src/main.rs"
/// e.g., "/abs/path/.kyco/worktrees/job-1-6/foo/bar.rs" -> "foo/bar.rs"
/// Returns None if the path is just the worktree directory itself (no file)
fn normalize_worktree_path(path: &str) -> String {
    // Look for worktree path pattern: .kyco/worktrees/
    const WORKTREE_DIR: &str = ".kyco/worktrees/";

    if let Some(idx) = path.find(WORKTREE_DIR) {
        // Get everything after ".kyco/worktrees/"
        let after_worktrees = &path[idx + WORKTREE_DIR.len()..];

        // Find the first slash after the branch name (job-XXX or job-XXX-Y)
        if let Some(slash_idx) = after_worktrees.find('/') {
            let file_path = &after_worktrees[slash_idx + 1..];
            // Only return if there's actually a file path after the worktree dir
            if !file_path.is_empty() {
                return file_path.to_string();
            }
        }
        // Path points to worktree directory itself, not a file
        return String::new();
    }

    // No worktree pattern found, return as-is but strip leading slash for consistency
    path.trim_start_matches('/').to_string()
}

/// Convert a Job to a JobStatsRecord for database storage
fn job_to_stats_record(job: &Job, agent_config: Option<&AgentConfigToml>) -> JobStatsRecord {
    let now = chrono::Utc::now().timestamp_millis();

    // Extract token usage from job (if available from bridge response)
    let (input_tokens, output_tokens, cache_read, cache_write) = extract_token_usage(job);

    // Use real cost if available, otherwise estimate from tokens using agent pricing
    let cost_usd = job.cost_usd.unwrap_or_else(|| {
        estimate_cost(input_tokens, output_tokens, cache_read, &job.agent_id, agent_config)
    });

    // Calculate duration
    let duration_ms = job
        .started_at
        .map(|start| {
            job.finished_at
                .unwrap_or(chrono::Utc::now())
                .signed_duration_since(start)
                .num_milliseconds() as u64
        })
        .unwrap_or(0);

    // Get file stats
    let (files_changed, lines_added, lines_removed) = job
        .stats
        .as_ref()
        .map(|s| (s.files_changed, s.lines_added, s.lines_removed))
        .unwrap_or((job.changed_files.len(), 0, 0));

    JobStatsRecord {
        job_id: job.id,
        session_id: job.bridge_session_id.clone(),
        mode: job.mode.clone(),
        agent_id: job.agent_id.clone(),
        agent_type: if job.agent_id.contains("codex") { "codex" } else { "claude" }.to_string(),
        status: format!("{:?}", job.status).to_lowercase(),
        input_tokens,
        output_tokens,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
        cost_usd,
        duration_ms,
        files_changed,
        lines_added,
        lines_removed,
        created_at: job.created_at.timestamp_millis(),
        started_at: job.started_at.map(|t| t.timestamp_millis()),
        finished_at: job.finished_at.map(|t| t.timestamp_millis()).or(Some(now)),
        workspace_id: job.workspace_id.map(|id| id.to_string()),
    }
}

/// Extract token usage from job (uses real values if available, falls back to estimate)
fn extract_token_usage(job: &Job) -> (u64, u64, u64, u64) {
    // Use real values from API response if available
    if let (Some(input), Some(output)) = (job.input_tokens, job.output_tokens) {
        return (
            input,
            output,
            job.cache_read_tokens.unwrap_or(0),
            job.cache_write_tokens.unwrap_or(0),
        );
    }

    // Fallback: estimate based on response length (~4 chars per token)
    let output_chars = job.full_response.as_ref().map(|r| r.len()).unwrap_or(0);
    let input_chars = job.sent_prompt.as_ref().map(|p| p.len()).unwrap_or(100);
    let input_tokens = (input_chars / 4) as u64;
    let output_tokens = (output_chars / 4) as u64;

    (input_tokens, output_tokens, 0, 0)
}

/// Estimate cost based on token usage
/// Uses agent-specific pricing if configured, otherwise falls back to defaults.
fn estimate_cost(
    input: u64,
    output: u64,
    cache_read: u64,
    agent_id: &str,
    agent_config: Option<&AgentConfigToml>,
) -> f64 {
    // Get pricing from agent config, or use defaults based on agent type
    let (price_input, price_cached, price_output) = if let Some(cfg) = agent_config {
        // Use configured pricing if available
        let is_codex = agent_id.contains("codex");
        let default_input = if is_codex { 1.75 } else { 3.00 };
        let default_cached = if is_codex { 0.175 } else { 0.30 };
        let default_output = if is_codex { 14.00 } else { 15.00 };

        (
            cfg.price_input.unwrap_or(default_input),
            cfg.price_cached_input.unwrap_or(default_cached),
            cfg.price_output.unwrap_or(default_output),
        )
    } else if agent_id.contains("codex") {
        // Codex/OpenAI GPT-5.2 defaults (per 1M tokens)
        (1.75, 0.175, 14.00)
    } else {
        // Claude Sonnet 4 defaults (per 1M tokens)
        (3.00, 0.30, 15.00)
    };

    // Calculate cost (prices are per 1M tokens)
    let fresh_input = input.saturating_sub(cache_read);
    let input_cost = fresh_input as f64 * price_input / 1_000_000.0;
    let cache_cost = cache_read as f64 * price_cached / 1_000_000.0;
    let output_cost = output as f64 * price_output / 1_000_000.0;

    input_cost + cache_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_worktree_path() {
        // Worktree paths should be normalized
        assert_eq!(
            normalize_worktree_path(".kyco/worktrees/job-5/src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            normalize_worktree_path("/Users/foo/project/.kyco/worktrees/job-123/foo/bar.rs"),
            "foo/bar.rs"
        );
        // Branch names with multiple dashes (job-1-6)
        assert_eq!(
            normalize_worktree_path("/Users/foo/.kyco/worktrees/job-1-6/src/lib.rs"),
            "src/lib.rs"
        );

        // Worktree directory without file should return empty string
        assert_eq!(
            normalize_worktree_path("/Users/foo/.kyco/worktrees/job-1-6"),
            ""
        );
        assert_eq!(
            normalize_worktree_path(".kyco/worktrees/job-5/"),
            ""
        );

        // Non-worktree paths should be returned as-is (with leading slash stripped)
        assert_eq!(
            normalize_worktree_path("/Users/foo/project/src/main.rs"),
            "Users/foo/project/src/main.rs"
        );
        assert_eq!(
            normalize_worktree_path("src/main.rs"),
            "src/main.rs"
        );
    }

    #[test]
    fn test_extract_file_path_from_args() {
        // file_path parameter
        let args = serde_json::json!({"file_path": "/path/to/file.rs"});
        assert_eq!(
            extract_file_path_from_args(Some(&args)),
            Some("/path/to/file.rs".to_string())
        );

        // path parameter (Glob, Grep)
        let args = serde_json::json!({"path": "src/main.rs", "pattern": "*.rs"});
        assert_eq!(
            extract_file_path_from_args(Some(&args)),
            Some("src/main.rs".to_string())
        );

        // Skip glob patterns
        let args = serde_json::json!({"path": "src/**/*.rs"});
        assert_eq!(extract_file_path_from_args(Some(&args)), None);

        // Skip directories
        let args = serde_json::json!({"path": "src/"});
        assert_eq!(extract_file_path_from_args(Some(&args)), None);

        // filePath for LSP
        let args = serde_json::json!({"filePath": "src/lib.rs", "line": 42});
        assert_eq!(
            extract_file_path_from_args(Some(&args)),
            Some("src/lib.rs".to_string())
        );
    }
}
