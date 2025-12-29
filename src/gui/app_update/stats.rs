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

        // Record file access for all paths found in tool args
        let file_paths = extract_file_paths_from_args(event.tool_args.as_ref());

        let access_type = match tool_name.as_str() {
            "Write" | "NotebookEdit" => FileAccessType::Write,
            "Edit" => FileAccessType::Edit,
            _ => FileAccessType::Read, // Default to read for Glob, Grep, Read, LSP, Bash, etc.
        };

        for file_path in file_paths {
            // Normalize worktree paths back to original paths
            let normalized_path = normalize_worktree_path(&file_path);

            // Skip if path is empty (e.g., worktree directory without file)
            if normalized_path.is_empty() {
                continue;
            }

            let file_record = FileStatsRecord {
                job_id,
                session_id: session_id.clone(),
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

/// Extract file paths from any tool arguments
/// Returns all file paths found (from explicit params and parsed from command strings)
fn extract_file_paths_from_args(args: Option<&serde_json::Value>) -> Vec<String> {
    let Some(args) = args else {
        return Vec::new();
    };

    let mut paths = Vec::new();

    // 1. Check explicit path parameters
    const PATH_PARAMS: &[&str] = &[
        "file_path",     // Read, Write, Edit
        "filePath",      // LSP
        "path",          // Glob, Grep
        "notebook_path", // NotebookEdit
    ];

    for param in PATH_PARAMS {
        if let Some(path) = args.get(*param).and_then(|v| v.as_str()) {
            if is_valid_file_path(path) {
                paths.push(path.to_string());
            }
        }
    }

    // 2. Parse command strings (Bash tool)
    if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
        paths.extend(extract_paths_from_command(command));
    }

    // 3. Scan all string values for path-like patterns
    if let Some(obj) = args.as_object() {
        for (key, value) in obj {
            // Skip already checked params and non-path fields
            if PATH_PARAMS.contains(&key.as_str()) || key == "command" || key == "tool_use_id" {
                continue;
            }
            if let Some(s) = value.as_str() {
                if looks_like_file_path(s) && is_valid_file_path(s) {
                    paths.push(s.to_string());
                }
            }
        }
    }

    paths
}

/// Check if a string looks like a file path (heuristic)
fn looks_like_file_path(s: &str) -> bool {
    // Must have reasonable length
    if s.len() < 3 || s.len() > 500 {
        return false;
    }

    // Skip strings with spaces (likely descriptions, not paths)
    if s.contains(' ') {
        return false;
    }

    // Skip system binary paths
    if s.starts_with("/bin/") || s.starts_with("/usr/bin/") || s.starts_with("/usr/local/bin/")
        || s.starts_with("/sbin/") || s.starts_with("/opt/")
    {
        return false;
    }

    // Skip shell executables by name
    let filename = s.rsplit('/').next().unwrap_or(s);
    const SHELL_BINARIES: &[&str] = &["zsh", "bash", "sh", "fish", "csh", "tcsh", "ksh", "dash"];
    if SHELL_BINARIES.contains(&filename) {
        return false;
    }

    // Common file extensions
    const EXTENSIONS: &[&str] = &[
        ".rs", ".ts", ".js", ".tsx", ".jsx", ".py", ".go", ".java", ".c", ".cpp", ".h",
        ".toml", ".json", ".yaml", ".yml", ".md", ".txt", ".sh", ".bash",
        ".html", ".css", ".scss", ".vue", ".svelte",
    ];

    // Check for file extension
    let has_extension = EXTENSIONS.iter().any(|ext| s.ends_with(ext));

    // Check for path-like structure
    let has_path_structure = s.contains('/') || s.starts_with("./") || s.starts_with("src/");

    has_extension || (has_path_structure && !s.starts_with("http"))
}

/// Extract file paths from a shell command string
fn extract_paths_from_command(command: &str) -> Vec<String> {
    let mut paths = Vec::new();

    // Split by common shell separators
    for token in command.split(|c: char| c.is_whitespace() || c == ';' || c == '|' || c == '&') {
        let token = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');

        if token.is_empty() {
            continue;
        }

        // Skip shell builtins and common commands
        const SKIP_TOKENS: &[&str] = &[
            "cd", "ls", "cat", "echo", "grep", "sed", "awk", "wc", "head", "tail",
            "git", "cargo", "npm", "node", "python", "rustc", "gcc", "make",
            "-l", "-n", "-q", "-v", "-r", "-f", "-p", "-a", "-e", "-i", "-c",
            "--offline", "--quiet", "--verbose", "--porcelain", "--oneline",
        ];

        if SKIP_TOKENS.contains(&token) || token.starts_with('-') {
            continue;
        }

        // Check if it looks like a file path
        if looks_like_file_path(token) && is_valid_file_path(token) {
            paths.push(token.to_string());
        }
    }

    paths
}

/// Validate that a path is a real file path (not a glob pattern or directory)
fn is_valid_file_path(path: &str) -> bool {
    !path.is_empty() && !path.ends_with('/') && !path.contains('*') && !path.contains('?')
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
        estimate_cost(
            input_tokens,
            output_tokens,
            cache_read,
            cache_write,
            &job.agent_id,
            agent_config,
        )
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
        workspace_path: job.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
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
    cache_write: u64,
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
    // - `input`: uncached input tokens
    // - `cache_read`: cached input tokens (discounted)
    // - `cache_write`: tokens written to cache (charged at input rate)
    let input_cost = (input + cache_write) as f64 * price_input / 1_000_000.0;
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
    fn test_extract_file_paths_from_args() {
        // file_path parameter
        let args = serde_json::json!({"file_path": "/path/to/file.rs"});
        assert_eq!(
            extract_file_paths_from_args(Some(&args)),
            vec!["/path/to/file.rs".to_string()]
        );

        // path parameter (Glob, Grep)
        let args = serde_json::json!({"path": "src/main.rs", "pattern": "*.rs"});
        assert_eq!(
            extract_file_paths_from_args(Some(&args)),
            vec!["src/main.rs".to_string()]
        );

        // Skip glob patterns
        let args = serde_json::json!({"path": "src/**/*.rs"});
        assert!(extract_file_paths_from_args(Some(&args)).is_empty());

        // Skip directories
        let args = serde_json::json!({"path": "src/"});
        assert!(extract_file_paths_from_args(Some(&args)).is_empty());

        // filePath for LSP
        let args = serde_json::json!({"filePath": "src/lib.rs", "line": 42});
        assert_eq!(
            extract_file_paths_from_args(Some(&args)),
            vec!["src/lib.rs".to_string()]
        );

        // Bash command with file path
        let args = serde_json::json!({"command": "wc -l src/lib.rs"});
        assert_eq!(
            extract_file_paths_from_args(Some(&args)),
            vec!["src/lib.rs".to_string()]
        );

        // Bash command with multiple file paths
        let args = serde_json::json!({"command": "sed -n '1,200p' src/lib.rs && cat Cargo.toml"});
        let paths = extract_file_paths_from_args(Some(&args));
        assert!(paths.contains(&"src/lib.rs".to_string()));
        assert!(paths.contains(&"Cargo.toml".to_string()));

        // Any string value that looks like a path
        let args = serde_json::json!({"some_field": "src/config/mod.rs", "other": "not a path"});
        assert_eq!(
            extract_file_paths_from_args(Some(&args)),
            vec!["src/config/mod.rs".to_string()]
        );
    }

    #[test]
    fn test_looks_like_file_path() {
        // Valid file paths
        assert!(looks_like_file_path("src/lib.rs"));
        assert!(looks_like_file_path("Cargo.toml"));
        assert!(looks_like_file_path("./foo/bar.ts"));
        assert!(looks_like_file_path("/abs/path/file.py"));
        assert!(looks_like_file_path("AGENTS.md"));

        // Not file paths
        assert!(!looks_like_file_path("git"));
        assert!(!looks_like_file_path("-l"));
        assert!(!looks_like_file_path("https://example.com/foo"));
        assert!(!looks_like_file_path("some random text"));
        assert!(!looks_like_file_path("Count lines in src/lib.rs")); // Description, not path

        // System binaries should be excluded
        assert!(!looks_like_file_path("/bin/zsh"));
        assert!(!looks_like_file_path("/usr/bin/bash"));
        assert!(!looks_like_file_path("/usr/local/bin/node"));
        assert!(!looks_like_file_path("zsh"));
        assert!(!looks_like_file_path("bash"));
    }
}
