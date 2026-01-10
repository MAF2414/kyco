//! Claude bridge adapter implementation.

use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

use super::super::client::BridgeClient;
use super::super::types::*;
use super::util::{bridge_cwd, extract_output_from_result, format_tool_call, parse_claude_permission_mode, parse_json_schema, resolve_prompt_paths};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

/// Maximum number of retries when connection drops
const MAX_CONNECTION_RETRIES: u32 = 3;
/// Maximum number of retries when rate limited (HTTP 429)
const MAX_RATE_LIMIT_RETRIES: u32 = 20;
/// Delay between connection-drop retries in milliseconds
const CONNECTION_RETRY_DELAY_MS: u64 = 2000;

static RETRY_AFTER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)retry[- ]after\s*[:=]?\s*(\d+)\s*(s|sec|secs|second|seconds|m|min|mins|minute|minutes)?")
        .expect("valid retry-after regex")
});

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

fn parse_retry_after_ms(message: &str) -> Option<u64> {
    let caps = RETRY_AFTER_RE.captures(message)?;
    let value: u64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2).map(|m| m.as_str().to_ascii_lowercase());
    match unit.as_deref() {
        None | Some("s") | Some("sec") | Some("secs") | Some("second") | Some("seconds") => Some(value.saturating_mul(1000)),
        Some("m") | Some("min") | Some("mins") | Some("minute") | Some("minutes") => Some(value.saturating_mul(60_000)),
        Some(_) => None,
    }
}

/// Calculate retry delay for rate limit errors with exponential backoff.
/// Pattern (seconds): 2, 4, 8, 16, 30, 60, 60, ...
fn rate_limit_delay_ms(attempt: u32, retry_after_ms: Option<u64>) -> u64 {
    if let Some(ms) = retry_after_ms {
        return ms.clamp(1_000, 60_000);
    }
    match attempt {
        1 => 2_000,
        2 => 4_000,
        3 => 8_000,
        4 => 16_000,
        5 => 30_000,
        _ => 60_000,
    }
}

fn add_jitter_ms(delay_ms: u64) -> u64 {
    let max_jitter = (delay_ms / 10).min(1_000);
    if max_jitter == 0 {
        return delay_ms;
    }
    let mut buf = [0u8; 8];
    if getrandom::getrandom(&mut buf).is_ok() {
        let r = u64::from_le_bytes(buf);
        delay_ms.saturating_add(r % (max_jitter + 1))
    } else {
        delay_ms
    }
}

/// Claude adapter using the SDK Bridge
///
/// Provides full Claude Agent SDK features:
/// - Hooks (PreToolUse, PostToolUse)
/// - Session resume
/// - Structured output
/// - Custom permissions
pub struct ClaudeBridgeAdapter {
    client: BridgeClient,
}

impl ClaudeBridgeAdapter {
    pub fn new() -> Self {
        Self { client: BridgeClient::new() }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self { client: BridgeClient::with_url(url) }
    }

    fn build_prompt(&self, job: &Job, _config: &AgentConfig) -> String {
        let paths = resolve_prompt_paths(job);

        // Use Claude's native skill invocation with /skill-name
        // The skill must be installed in .claude/skills/ for Claude to find it
        let mut prompt = format!("/{}", job.skill);

        // Add file context
        prompt.push_str(&format!(" on file {}:{}", paths.file_path, job.source_line));

        // Add IDE context if available
        if !paths.ide_context.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&paths.ide_context);
        }

        // Add user description if provided
        if let Some(desc) = &job.description {
            if !desc.is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(desc);
            }
        }

        prompt
    }

    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_skill_template(&job.skill);
        let mut system_prompt = template.system_prompt.unwrap_or_default();

        if let Some(wt_path) = &job.git_worktree_path {
            system_prompt.push_str(&format!(
                "\n\n## Working Directory\n\
                **IMPORTANT:** You are working in an isolated Git worktree at:\n`{}`\n\n\
                - All file paths in the task are relative to this worktree.\n\
                - Do NOT edit files outside this directory.\n\
                - When you have completed the task, commit your changes with a descriptive message.\n\
                - Do NOT push.",
                wt_path.display()
            ));
        }

        if let Some(schema) = &config.output_schema {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(schema);
        }

        if let Some(ref state_prompt) = template.state_prompt {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(state_prompt);
        } else if !template.output_states.is_empty() {
            system_prompt.push_str("\n\n## Output State\nWhen you complete this task, indicate the outcome by stating one of the following:\n");
            for state in &template.output_states {
                system_prompt.push_str(&format!("- state: {}\n", state));
            }
            system_prompt.push_str(&format!("\nExample: \"state: {}\" (at the end of your response)", &template.output_states[0]));
        }

        if system_prompt.is_empty() { None } else { Some(system_prompt) }
    }

    /// Build a continuation request for retrying a dropped connection.
    fn build_continue_request(&self, session_id: String, cwd: String, config: &AgentConfig) -> ClaudeQueryRequest {
        ClaudeQueryRequest {
            prompt: "continue".to_string(),
            images: None,
            cwd,
            session_id: Some(session_id),
            fork_session: None,
            permission_mode: Some(parse_claude_permission_mode(&config.permission_mode)),
            agents: None,
            allowed_tools: None,
            disallowed_tools: None,
            env: None,
            mcp_servers: None,
            system_prompt: None,
            system_prompt_mode: None,
            setting_sources: None,
            plugins: None,
            max_turns: if config.max_turns > 0 { Some(config.max_turns) } else { None },
            max_thinking_tokens: None,
            model: config.model.clone(),
            output_schema: None,
            kyco_callback_url: None,
            hooks: None,
        }
    }

    /// Build request, taking ownership of prompt and cwd to avoid clones.
    /// Config fields are cloned only when non-empty (None avoids allocation).
    fn build_request(&self, job: &Job, config: &AgentConfig, prompt: String, cwd: String) -> ClaudeQueryRequest {
        // Helper: clone only if non-empty, otherwise None (avoids empty collection allocation)
        fn clone_if_non_empty<T: Clone>(v: &[T]) -> Option<Vec<T>> {
            if v.is_empty() { None } else { Some(v.to_vec()) }
        }
        fn clone_map_if_non_empty<K: Clone + Eq + std::hash::Hash, V: Clone>(
            m: &std::collections::HashMap<K, V>,
        ) -> Option<std::collections::HashMap<K, V>> {
            if m.is_empty() { None } else { Some(m.clone()) }
        }

        ClaudeQueryRequest {
            prompt,
            images: None,
            cwd,
            session_id: job.bridge_session_id.clone(),
            fork_session: None,
            permission_mode: Some(parse_claude_permission_mode(&config.permission_mode)),
            agents: clone_map_if_non_empty(&config.agents),
            allowed_tools: clone_if_non_empty(&config.allowed_tools),
            disallowed_tools: clone_if_non_empty(&config.disallowed_tools),
            env: clone_map_if_non_empty(&config.env),
            mcp_servers: clone_map_if_non_empty(&config.mcp_servers),
            system_prompt: self.build_system_prompt(job, config),
            system_prompt_mode: Some(match config.system_prompt_mode {
                crate::SystemPromptMode::Replace => "replace",
                crate::SystemPromptMode::Append | crate::SystemPromptMode::ConfigOverride => "append",
            }.into()),
            setting_sources: Some(vec!["user".into(), "project".into(), "local".into()]),
            plugins: {
                let plugins: Vec<ClaudePlugin> = config.plugins.iter()
                    .map(|p| p.trim()).filter(|p| !p.is_empty())
                    .map(|path| ClaudePlugin { plugin_type: ClaudePluginType::Local, path: path.to_string() })
                    .collect();
                if plugins.is_empty() { None } else { Some(plugins) }
            },
            max_turns: if config.max_turns > 0 { Some(config.max_turns) } else { None },
            max_thinking_tokens: None,
            model: config.model.clone(),
            output_schema: parse_json_schema(config.structured_output_schema.as_deref()),
            kyco_callback_url: None,
            hooks: None,
        }
    }
}

impl Default for ClaudeBridgeAdapter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl AgentRunner for ClaudeBridgeAdapter {
    async fn run(&self, job: &Job, worktree: &Path, config: &AgentConfig, event_tx: mpsc::Sender<LogEvent>) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config);
        let cwd = bridge_cwd(worktree);

        let _ = event_tx.send(LogEvent::system(format!("Starting Claude SDK job #{}", job_id)).for_job(job_id)).await;
        let _ = event_tx.send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id)).await;

        // Clone prompt for sent_prompt before moving into request
        let sent_prompt = prompt.clone();
        let initial_request = self.build_request(job, config, prompt, cwd.clone());
        let mut result = AgentResult {
            success: false, error: None, changed_files: Vec::new(), cost_usd: None,
            input_tokens: None, output_tokens: None, cache_read_tokens: None, cache_write_tokens: None,
            duration_ms: None, sent_prompt: Some(sent_prompt), output_text: None, session_id: None,
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
            let mut rate_limited_retry_after_ms: Option<u64> = None;
            let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
            let client = self.client.clone();

            // Build request: initial or continuation
            let request = if use_continue_request {
                captured_session_id
                    .as_ref()
                    .map(|sid| self.build_continue_request(sid.clone(), cwd.clone(), config))
                    .unwrap_or_else(|| initial_request.clone())
            } else {
                initial_request.clone()
            };

            tokio::task::spawn_blocking(move || {
                match client.claude_query(&request) {
                    Ok(events) => { for ev in events { if tx.blocking_send(ev.map_err(|e| e.to_string())).is_err() { break; } } }
                    Err(e) => { let _ = tx.blocking_send(Err(e.to_string())); }
                }
            });

            while let Some(event_result) = rx.recv().await {
                let event = match event_result {
                    Ok(e) => e,
                    Err(e) => {
                        if is_rate_limited(None, &e) {
                            rate_limited_retry_after_ms = parse_retry_after_ms(&e);
                            rate_limited_message = Some(e);
                        } else {
                            let message = format!("Bridge event stream error: {}", e);
                            result.error = Some(message.clone());
                            let _ = event_tx.send(LogEvent::error(message).for_job(job_id)).await;
                        }
                        break;
                    }
                };

                match event {
                    BridgeEvent::SessionStart { session_id, model, tools, .. } => {
                        let preview: String = session_id.get(..12).unwrap_or(&session_id).into();
                        let _ = event_tx.send(LogEvent::system(format!("Session started: {} tools available, model: {} (session: {})", tools.len(), model, preview))
                            .with_tool_args(serde_json::json!({ "session_id": &session_id })).for_job(job_id)).await;
                        captured_session_id = Some(session_id);
                    }
                    BridgeEvent::Text { content, partial, .. } => {
                        if !partial { output_text.push_str(&content); output_text.push('\n'); }
                        let _ = event_tx.send(LogEvent::text(content).for_job(job_id)).await;
                    }
                    BridgeEvent::Reasoning { content, .. } => {
                        // Log reasoning as thought (not added to output_text)
                        let _ = event_tx.send(LogEvent::thought(content).for_job(job_id)).await;
                    }
                    BridgeEvent::ToolUse { tool_name, mut tool_input, tool_use_id, .. } => {
                        let formatted = format_tool_call(&tool_name, &tool_input);
                        if let Some(obj) = tool_input.as_object_mut() {
                            obj.insert("tool_use_id".into(), serde_json::json!(tool_use_id));
                        }
                        let _ = event_tx.send(LogEvent::tool_call(tool_name, formatted)
                            .with_tool_args(tool_input).for_job(job_id)).await;
                    }
                    BridgeEvent::ToolResult { success, output, files_changed, .. } => {
                        if let Some(files) = files_changed { for f in files { result.changed_files.push(std::path::PathBuf::from(f)); } }
                        let _ = event_tx.send(LogEvent::tool_output("tool", if success { output } else { format!("Error: {}", output) }).for_job(job_id)).await;
                    }
                    BridgeEvent::Error { message, code, .. } => {
                        if is_rate_limited(code.as_deref(), &message) {
                            rate_limited_retry_after_ms = parse_retry_after_ms(&message);
                            rate_limited_message = Some(message);
                        } else {
                            let _ = event_tx.send(LogEvent::error(&message).for_job(job_id)).await;
                            result.error = Some(message);
                        }
                        break;
                    }
                    BridgeEvent::SessionComplete { success, cost_usd, duration_ms, usage, result: sr, .. } => {
                        received_session_complete = true;
                        result.success = success; result.cost_usd = cost_usd; result.duration_ms = Some(duration_ms); structured_result = sr;
                        if let Some(ref u) = usage {
                            result.input_tokens = Some(u.effective_fresh_input());
                            result.output_tokens = Some(u.output_tokens);
                            result.cache_read_tokens = Some(u.effective_cache_read());
                            result.cache_write_tokens = u.cache_write_tokens;
                        }
                        let usage_info = usage.map(|u| format!(", {} input + {} output tokens", u.input_tokens, u.output_tokens)).unwrap_or_default();
                        let _ = event_tx.send(LogEvent::system(format!("Completed: {} (cost: ${:.4}, duration: {}ms{})", if success { "success" } else { "failed" }, cost_usd.unwrap_or(0.0), duration_ms, usage_info)).for_job(job_id)).await;
                        break;
                    }
                    BridgeEvent::ToolApprovalNeeded { request_id, session_id, tool_name, tool_input, .. } => {
                        tracing::info!("⚠️ Received ToolApprovalNeeded from Bridge: tool={}, request_id={}, job_id={}", tool_name, request_id, job_id);
                        let _ = event_tx.send(LogEvent::permission(format!("Tool approval needed: {}", tool_name))
                            .with_tool_args(serde_json::json!({ "request_id": request_id, "session_id": session_id, "tool_name": tool_name, "tool_input": tool_input })).for_job(job_id)).await;
                    }
                    BridgeEvent::HookPreToolUse { tool_name, tool_input, tool_use_id, .. } => {
                        let formatted = format!("[hook PreToolUse] {}", format_tool_call(&tool_name, &tool_input));
                        let _ = event_tx.send(LogEvent::tool_call(tool_name, formatted)
                            .with_tool_args(serde_json::json!({ "tool_use_id": tool_use_id })).for_job(job_id)).await;
                    }
                    BridgeEvent::Heartbeat { .. } => {}
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

                let base_delay_ms = rate_limit_delay_ms(rate_limit_retries, rate_limited_retry_after_ms);
                let delay_ms = add_jitter_ms(base_delay_ms);
                let delay_s = (delay_ms + 999) / 1000;
                let _ = event_tx
                    .send(
                        LogEvent::system(format!(
                            "Rate limited, retrying in {}s ({}/{})...",
                            delay_s, rate_limit_retries, MAX_RATE_LIMIT_RETRIES
                        ))
                        .for_job(job_id),
                    )
                    .await;
                use_continue_request = captured_session_id.is_some();
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }

            if result.error.is_some() {
                break;
            }

            // Check if we can retry
            if connection_retries >= MAX_CONNECTION_RETRIES {
                result.error = Some("Claude session ended unexpectedly (no completion event received)".to_string());
                let _ = event_tx.send(LogEvent::error("Claude session ended unexpectedly").for_job(job_id)).await;
                break;
            }

            connection_retries += 1;
            use_continue_request = captured_session_id.is_some();
            let _ = event_tx
                .send(
                    LogEvent::system(format!(
                        "Connection dropped, retrying ({}/{})...",
                        connection_retries, MAX_CONNECTION_RETRIES
                    ))
                    .for_job(job_id),
                )
                .await;
            tokio::time::sleep(Duration::from_millis(CONNECTION_RETRY_DELAY_MS)).await;
        }

        if !output_text.is_empty() { result.output_text = Some(output_text); }
        extract_output_from_result(&mut result.output_text, structured_result);
        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str { "claude" }
    fn is_available(&self) -> bool { self.client.health_check().is_ok() }
}

#[cfg(test)]
mod tests {
    use super::{is_rate_limited, parse_retry_after_ms, rate_limit_delay_ms};

    #[test]
    fn detects_rate_limit_by_code() {
        assert!(is_rate_limited(Some("429"), "anything"));
        assert!(is_rate_limited(Some("rate_limit"), "anything"));
        assert!(is_rate_limited(Some("RATE_LIMIT"), "anything"));
    }

    #[test]
    fn detects_rate_limit_by_message() {
        assert!(is_rate_limited(None, "HTTP 429 Too Many Requests"));
        assert!(is_rate_limited(None, "Rate limit exceeded"));
        assert!(is_rate_limited(None, "Too many requests"));
    }

    #[test]
    fn parses_retry_after_ms() {
        assert_eq!(parse_retry_after_ms("Retry-After: 5"), Some(5_000));
        assert_eq!(parse_retry_after_ms("retry after 2s"), Some(2_000));
        assert_eq!(parse_retry_after_ms("retry-after=3 mins"), Some(180_000));
        assert_eq!(parse_retry_after_ms("no hint here"), None);
    }

    #[test]
    fn rate_limit_delay_clamps_retry_after() {
        assert_eq!(rate_limit_delay_ms(1, Some(500)), 1_000);
        assert_eq!(rate_limit_delay_ms(1, Some(90_000)), 60_000);
    }
}
