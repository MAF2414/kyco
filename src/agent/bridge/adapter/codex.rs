//! Codex bridge adapter implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

use super::super::client::BridgeClient;
use super::super::types::*;
use super::util::{bridge_cwd, extract_output_from_result, format_tool_call, parse_json_schema, resolve_prompt_paths};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

/// Maximum number of retries when connection drops
const MAX_RETRIES: u32 = 15;
/// Maximum number of retries when rate limited (HTTP 429)
const MAX_RATE_LIMIT_RETRIES: u32 = 20;

fn is_rate_limited(code: Option<&str>, message: &str) -> bool {
    if code.is_some_and(|c| c.eq_ignore_ascii_case("429") || c.eq_ignore_ascii_case("rate_limit")) {
        return true;
    }
    if message.contains("429") {
        return true;
    }
    let msg = message.to_ascii_lowercase();
    msg.contains("rate limit") || msg.contains("too many requests")
}

/// Check if an error message indicates a retriable connection issue
fn is_retriable_connection_error(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    msg.contains("reconnect")
        || msg.contains("connection")
        || msg.contains("network")
        || msg.contains("timeout")
        || msg.contains("reset")
        || msg.contains("econnreset")
        || msg.contains("epipe")
        || msg.contains("socket")
        || msg.contains("closed")
        || msg.contains("disconnected")
}

/// Calculate retry delay with exponential backoff (capped at 30s)
/// Pattern: 1s, 2s, 4s, 8s, 10s, 20s, 30s, 30s, ...
fn retry_delay_ms(attempt: u32) -> u64 {
    match attempt {
        1 => 1_000,
        2 => 2_000,
        3 => 4_000,
        4 => 8_000,
        5 => 10_000,
        6 => 20_000,
        _ => 30_000, // Cap at 30 seconds
    }
}

/// Codex adapter using the SDK Bridge
pub struct CodexBridgeAdapter {
    client: BridgeClient,
}

impl CodexBridgeAdapter {
    pub fn new() -> Self {
        Self { client: BridgeClient::new() }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self { client: BridgeClient::with_url(url) }
    }

    /// Find skill directory and list its files
    fn find_skill_files(&self, job: &Job, worktree: &Path) -> (Option<String>, Vec<String>) {
        // Try workspace-local first, then global
        let skill_dirs = [
            worktree.join(".codex/skills").join(&job.mode),
            dirs::home_dir()
                .map(|h| h.join(".codex/skills").join(&job.mode))
                .unwrap_or_default(),
        ];

        for skill_dir in skill_dirs {
            if skill_dir.exists() && skill_dir.is_dir() {
                let dir_path = skill_dir.display().to_string();
                let mut files = Vec::new();

                if let Ok(entries) = fs::read_dir(&skill_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(name) = path.file_name() {
                                files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
                files.sort();
                return (Some(dir_path), files);
            }
        }

        (None, Vec::new())
    }

    pub(super) fn build_prompt(&self, job: &Job, config: &AgentConfig, worktree: &Path) -> String {
        let paths = resolve_prompt_paths(job);
        let template = config.get_skill_template(&job.mode);

        // Find skill directory and files
        let (skill_dir, skill_files) = self.find_skill_files(job, worktree);

        // Build prompt with skill instructions for Codex
        // (Codex doesn't have native skill loading like Claude's /skill command)
        let mut prompt = String::new();

        // Add system prompt / skill instructions first
        if let Some(system_prompt) = template.system_prompt.as_deref() {
            let system_prompt = system_prompt.trim();
            if !system_prompt.is_empty() {
                prompt.push_str("## Skill Instructions\n\n");
                prompt.push_str(system_prompt);
                prompt.push_str("\n\n");
            }
        }

        // Add skill directory info if found
        if let Some(dir) = &skill_dir {
            prompt.push_str(&format!("## Skill Directory\n\nSkill '{}' is located at: `{}`\n", job.mode, dir));
            if !skill_files.is_empty() {
                prompt.push_str("\nFiles in skill directory:\n");
                for file in &skill_files {
                    prompt.push_str(&format!("- `{}`\n", file));
                }
            }
            prompt.push_str("\n");
        }

        // Add the task
        prompt.push_str(&format!(
            "## Task\n\nExecute the '{}' skill on file `{}` at line {}.\n",
            job.mode, paths.file_path, job.source_line
        ));

        // Add IDE context if available
        if !paths.ide_context.is_empty() {
            prompt.push_str("\n");
            prompt.push_str(&paths.ide_context);
        }

        // Add user description if provided
        if let Some(desc) = &job.description {
            if !desc.is_empty() {
                prompt.push_str("\n\n## User Request\n\n");
                prompt.push_str(desc);
            }
        }

        prompt
    }

    /// Build a continuation request for retrying a dropped connection.
    fn build_continue_request(&self, thread_id: String, cwd: String, config: &AgentConfig) -> CodexQueryRequest {
        CodexQueryRequest {
            prompt: "continue".to_string(),
            images: None,
            cwd,
            thread_id: Some(thread_id),
            sandbox: config.sandbox.clone().or_else(|| Some("workspace-write".to_string())),
            env: None,
            output_schema: None,
            model: config.model.clone(),
            effort: None,
            approval_policy: None,
            skip_git_repo_check: None,
        }
    }
}

impl Default for CodexBridgeAdapter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl AgentRunner for CodexBridgeAdapter {
    async fn run(&self, job: &Job, worktree: &Path, config: &AgentConfig, event_tx: mpsc::Sender<LogEvent>) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config, worktree);
        let cwd = bridge_cwd(worktree);

        let _ = event_tx.send(LogEvent::system(format!("Starting Codex SDK job #{}", job_id)).for_job(job_id)).await;

        let initial_request = CodexQueryRequest {
            prompt: prompt.clone(),
            images: None,
            cwd: cwd.clone(),
            thread_id: job.bridge_session_id.clone(),
            sandbox: config.sandbox.clone().or_else(|| Some("workspace-write".to_string())),
            env: if config.env.is_empty() { None } else { Some(config.env.clone()) },
            output_schema: parse_json_schema(config.structured_output_schema.as_deref()),
            model: config.model.clone(), effort: None, approval_policy: None, skip_git_repo_check: None,
        };

        let mut result = AgentResult {
            success: false, error: None, changed_files: Vec::new(), cost_usd: None,
            input_tokens: None, output_tokens: None, cache_read_tokens: None, cache_write_tokens: None,
            duration_ms: None, sent_prompt: Some(prompt), output_text: None, session_id: None,
        };

        let mut output_text = String::new();
        let mut captured_session_id: Option<String> = job.bridge_session_id.clone();
        let mut structured_result: Option<serde_json::Value> = None;
        let mut connection_retries = 0u32;
        let mut rate_limit_retries = 0u32;
        let mut use_continue_request = false;

        loop {
            let mut received_session_complete = false;
            let mut rate_limited_message: Option<String> = None;
            let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
            let client = self.client.clone();

            // Build request: initial or continuation
            let request = if use_continue_request {
                captured_session_id
                    .as_ref()
                    .map(|tid| self.build_continue_request(tid.clone(), cwd.clone(), config))
                    .unwrap_or_else(|| initial_request.clone())
            } else {
                initial_request.clone()
            };

            tokio::task::spawn_blocking(move || match client.codex_query(&request) {
                Ok(events) => { for ev in events { if tx.blocking_send(ev.map_err(|e| e.to_string())).is_err() { break; } } }
                Err(e) => { let _ = tx.blocking_send(Err(e.to_string())); }
            });

            while let Some(event_result) = rx.recv().await {
                let event = match event_result {
                    Ok(e) => e,
                    Err(e) => {
                        if is_rate_limited(None, &e) {
                            rate_limited_message = Some(e);
                        } else if is_retriable_connection_error(&e) {
                            // Log as system message, don't set result.error so outer loop retries
                            let _ = event_tx.send(LogEvent::system(format!("Connection issue: {}", e)).for_job(job_id)).await;
                        } else {
                            let message = format!("Bridge event stream error: {}", e);
                            result.error = Some(message.clone());
                            let _ = event_tx.send(LogEvent::error(message).for_job(job_id)).await;
                        }
                        break;
                    }
                };

                match event {
                    BridgeEvent::SessionStart { session_id, .. } => {
                        captured_session_id = Some(session_id.clone());
                        let _ = event_tx.send(LogEvent::system("Codex thread started").with_tool_args(serde_json::json!({ "session_id": session_id })).for_job(job_id)).await;
                    }
                    BridgeEvent::Text { content, partial, .. } => {
                        // Detect reasoning that comes as text (Codex SDK sends [Reasoning] prefix)
                        if content.starts_with("[Reasoning]") {
                            // Strip the [Reasoning] prefix and log as thought
                            let reasoning_content = content.strip_prefix("[Reasoning]").unwrap_or(&content).trim();
                            let _ = event_tx.send(LogEvent::thought(reasoning_content.to_string()).for_job(job_id)).await;
                            // Don't add reasoning to output_text
                        } else {
                            if !partial { output_text.push_str(&content); output_text.push('\n'); }
                            let _ = event_tx.send(LogEvent::text(content).for_job(job_id)).await;
                        }
                    }
                    BridgeEvent::Reasoning { content, .. } => {
                        // Log reasoning as thought (not added to output_text)
                        let _ = event_tx.send(LogEvent::thought(content).for_job(job_id)).await;
                    }
                    BridgeEvent::ToolUse { tool_name, tool_input, tool_use_id, .. } => {
                        let mut args = tool_input.clone();
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("tool_use_id".to_string(), serde_json::json!(tool_use_id));
                        }
                        let _ = event_tx.send(LogEvent::tool_call(tool_name.clone(), format_tool_call(&tool_name, &tool_input))
                            .with_tool_args(args).for_job(job_id)).await;
                    }
                    BridgeEvent::ToolResult { output, files_changed, .. } => {
                        if let Some(files) = files_changed { for f in files { result.changed_files.push(std::path::PathBuf::from(f)); } }
                        let _ = event_tx.send(LogEvent::tool_output("tool", output).for_job(job_id)).await;
                    }
                    BridgeEvent::Error { message, code, .. } => {
                        if is_rate_limited(code.as_deref(), &message) {
                            rate_limited_message = Some(message);
                        } else if is_retriable_connection_error(&message) {
                            // Log as system message, don't set result.error so outer loop retries
                            let _ = event_tx.send(LogEvent::system(format!("Connection issue: {}", message)).for_job(job_id)).await;
                        } else {
                            result.error = Some(message.clone());
                            let _ = event_tx.send(LogEvent::error(message).for_job(job_id)).await;
                        }
                        break;
                    }
                    BridgeEvent::SessionComplete { success, duration_ms, usage, result: sr, .. } => {
                        received_session_complete = true;
                        result.success = success; result.duration_ms = Some(duration_ms); structured_result = sr;
                        if let Some(ref u) = usage {
                            result.input_tokens = Some(u.input_tokens);
                            result.output_tokens = Some(u.output_tokens);
                            result.cache_read_tokens = Some(u.effective_cache_read());
                            result.cache_write_tokens = u.cache_write_tokens;
                        }
                        let usage_info = usage.map(|u| format!(", {} tokens", u.input_tokens + u.output_tokens)).unwrap_or_default();
                        let _ = event_tx.send(LogEvent::system(format!("Completed: {} (duration: {}ms{})", if success { "success" } else { "failed" }, duration_ms, usage_info)).for_job(job_id)).await;
                        break;
                    }
                    _ => {}
                }
            }

            // Stream ended - check if we should retry
            if received_session_complete {
                break;
            }

            if let Some(message) = rate_limited_message.take() {
                rate_limit_retries += 1;
                if rate_limit_retries > MAX_RATE_LIMIT_RETRIES {
                    let final_message = format!(
                        "Rate limited (HTTP 429) too many times ({}/{}): {}",
                        rate_limit_retries, MAX_RATE_LIMIT_RETRIES, message
                    );
                    result.error = Some(final_message.clone());
                    let _ = event_tx.send(LogEvent::error(&final_message).for_job(job_id)).await;
                    break;
                }

                let delay = retry_delay_ms(rate_limit_retries);
                let _ = event_tx
                    .send(
                        LogEvent::system(format!(
                            "Rate limited, retrying in {}s ({}/{})...",
                            delay / 1000,
                            rate_limit_retries,
                            MAX_RATE_LIMIT_RETRIES
                        ))
                        .for_job(job_id),
                    )
                    .await;
                use_continue_request = captured_session_id.is_some();
                tokio::time::sleep(Duration::from_millis(delay)).await;
                continue;
            }

            if result.error.is_some() {
                break;
            }

            if connection_retries >= MAX_RETRIES {
                result.error = Some("Codex session ended unexpectedly (no completion event received)".to_string());
                let _ = event_tx.send(LogEvent::error("Codex session ended unexpectedly").for_job(job_id)).await;
                break;
            }

            connection_retries += 1;
            use_continue_request = captured_session_id.is_some();
            let delay = retry_delay_ms(connection_retries);
            let _ = event_tx
                .send(
                    LogEvent::system(format!(
                        "Connection dropped, retrying in {}s ({}/{})...",
                        delay / 1000,
                        connection_retries,
                        MAX_RETRIES
                    ))
                    .for_job(job_id),
                )
                .await;
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        if !output_text.is_empty() { result.output_text = Some(output_text); }
        extract_output_from_result(&mut result.output_text, structured_result);
        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str { "codex" }
    fn is_available(&self) -> bool { self.client.health_check().is_ok() }
}

#[cfg(test)]
mod tests {
    use super::{is_rate_limited, is_retriable_connection_error, CodexBridgeAdapter};
    use crate::{AgentConfig, Job, ScopeDefinition};
    use std::path::{Path, PathBuf};

    fn create_test_job(mode: &str, description: Option<&str>, source_file: &str, source_line: usize) -> Job {
        Job::new(1, mode.to_string(), ScopeDefinition::file(PathBuf::from(source_file)), format!("{}:{}", source_file, source_line),
            description.map(|s| s.to_string()), "codex".to_string(), PathBuf::from(source_file), source_line, None)
    }

    #[test]
    fn codex_build_prompt_uses_native_skill_invocation() {
        let adapter = CodexBridgeAdapter::new();
        let config = AgentConfig::codex_default();
        let job = create_test_job("refactor", Some("fix the bug"), "src/main.rs", 42);
        let prompt = adapter.build_prompt(&job, &config, Path::new("."));

        // Should include task section with skill name and file context
        assert!(prompt.contains("## Task"), "Expected Task section, got: {}", prompt);
        assert!(prompt.contains("Execute the 'refactor' skill"), "Expected skill execution instruction, got: {}", prompt);
        assert!(prompt.contains("src/main.rs"), "Expected file context, got: {}", prompt);
        assert!(prompt.contains("line 42"), "Expected line context, got: {}", prompt);
        assert!(prompt.contains("fix the bug"), "Expected description to be included, got: {}", prompt);
    }

    #[test]
    fn detects_rate_limit_code() {
        assert!(is_rate_limited(Some("429"), "anything"));
        assert!(is_rate_limited(Some("rate_limit"), "anything"));
        assert!(is_rate_limited(Some("RATE_LIMIT"), "anything"));
    }

    #[test]
    fn detects_retriable_connection_errors() {
        // "Reconnecting... 1/5" is the actual message from Codex SDK
        assert!(is_retriable_connection_error("Reconnecting... 1/5"));
        assert!(is_retriable_connection_error("Connection lost"));
        assert!(is_retriable_connection_error("Network error"));
        assert!(is_retriable_connection_error("timeout"));
        assert!(is_retriable_connection_error("ECONNRESET"));
        assert!(is_retriable_connection_error("socket closed"));
        // Should NOT match unrelated errors
        assert!(!is_retriable_connection_error("Invalid API key"));
        assert!(!is_retriable_connection_error("File not found"));
    }
}
