//! Claude bridge adapter implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::sync::mpsc;

use super::super::client::BridgeClient;
use super::super::types::*;
use super::util::{bridge_cwd, extract_output_from_result, format_tool_call, parse_claude_permission_mode, parse_json_schema, resolve_prompt_paths};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

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

    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);
        let paths = resolve_prompt_paths(job);

        template.prompt_template
            .replace("{file}", &paths.file_path)
            .replace("{line}", &job.source_line.to_string())
            .replace("{target}", &paths.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", job.description.as_deref().unwrap_or(""))
            .replace("{scope_type}", "file")
            .replace("{ide_context}", &paths.ide_context)
    }

    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_mode_template(&job.mode);
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

        // Clone prompt for sent_prompt before moving into request (eliminates one clone vs prior approach)
        let sent_prompt = prompt.clone();
        let request = self.build_request(job, config, prompt, cwd);
        let mut result = AgentResult {
            success: false, error: None, changed_files: Vec::new(), cost_usd: None,
            input_tokens: None, output_tokens: None, cache_read_tokens: None, cache_write_tokens: None,
            duration_ms: None, sent_prompt: Some(sent_prompt), output_text: None, session_id: None,
        };

        let mut output_text = String::new();
        let mut captured_session_id: Option<String> = None;
        let mut structured_result: Option<serde_json::Value> = None;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
        let client = self.client.clone();

        tokio::task::spawn_blocking(move || {
            match client.claude_query(&request) {
                Ok(events) => { for ev in events { if tx.blocking_send(ev.map_err(|e| e.to_string())).is_err() { break; } } }
                Err(e) => { let _ = tx.blocking_send(Err(e.to_string())); }
            }
        });

        while let Some(event_result) = rx.recv().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => { let _ = event_tx.send(LogEvent::error(format!("Event parse error: {}", e)).for_job(job_id)).await; continue; }
            };

            match event {
                // Take ownership of session_id; compute preview before moving
                BridgeEvent::SessionStart { session_id, model, tools, .. } => {
                    let preview: String = session_id.get(..12).unwrap_or(&session_id).into();
                    let _ = event_tx.send(LogEvent::system(format!("Session started: {} tools available, model: {} (session: {})", tools.len(), model, preview))
                        .with_tool_args(serde_json::json!({ "session_id": &session_id })).for_job(job_id)).await;
                    captured_session_id = Some(session_id); // Move instead of clone
                }
                BridgeEvent::Text { content, partial, .. } => {
                    if !partial { output_text.push_str(&content); output_text.push('\n'); }
                    let _ = event_tx.send(LogEvent::text(content).for_job(job_id)).await;
                }
                // Take ownership of tool_input and modify in-place (eliminates clone)
                BridgeEvent::ToolUse { tool_name, mut tool_input, tool_use_id, .. } => {
                    // Format before modifying tool_input
                    let formatted = format_tool_call(&tool_name, &tool_input);
                    // Merge tool_use_id into tool_input in-place
                    if let Some(obj) = tool_input.as_object_mut() {
                        obj.insert("tool_use_id".into(), serde_json::json!(tool_use_id));
                    }
                    let _ = event_tx.send(LogEvent::tool_call(tool_name, formatted)
                        .with_tool_args(tool_input) // Move instead of clone
                        .for_job(job_id)).await;
                }
                BridgeEvent::ToolResult { success, output, files_changed, .. } => {
                    if let Some(files) = files_changed { for f in files { result.changed_files.push(std::path::PathBuf::from(f)); } }
                    let _ = event_tx.send(LogEvent::tool_output("tool", if success { output } else { format!("Error: {}", output) }).for_job(job_id)).await;
                }
                // Take ownership of message; log first then store (eliminates clone)
                BridgeEvent::Error { message, .. } => {
                    let _ = event_tx.send(LogEvent::error(&message).for_job(job_id)).await;
                    result.error = Some(message); // Move after borrow ends
                }
                BridgeEvent::SessionComplete { success, cost_usd, duration_ms, usage, result: sr, .. } => {
                    result.success = success; result.cost_usd = cost_usd; result.duration_ms = Some(duration_ms); structured_result = sr;
                    if let Some(ref u) = usage {
                        result.input_tokens = Some(u.input_tokens);
                        result.output_tokens = Some(u.output_tokens);
                        result.cache_read_tokens = Some(u.effective_cache_read());
                        result.cache_write_tokens = u.cache_write_tokens;
                    }
                    let usage_info = usage.map(|u| format!(", {} input + {} output tokens", u.input_tokens, u.output_tokens)).unwrap_or_default();
                    let _ = event_tx.send(LogEvent::system(format!("Completed: {} (cost: ${:.4}, duration: {}ms{})", if success { "success" } else { "failed" }, cost_usd.unwrap_or(0.0), duration_ms, usage_info)).for_job(job_id)).await;
                }
                BridgeEvent::ToolApprovalNeeded { request_id, session_id, tool_name, tool_input, .. } => {
                    tracing::info!("⚠️ Received ToolApprovalNeeded from Bridge: tool={}, request_id={}, job_id={}", tool_name, request_id, job_id);
                    let _ = event_tx.send(LogEvent::permission(format!("Tool approval needed: {}", tool_name))
                        .with_tool_args(serde_json::json!({ "request_id": request_id, "session_id": session_id, "tool_name": tool_name, "tool_input": tool_input })).for_job(job_id)).await;
                }
                // Take ownership and use reference for format_tool_call (eliminates clone)
                BridgeEvent::HookPreToolUse { tool_name, tool_input, tool_use_id, .. } => {
                    let formatted = format!("[hook PreToolUse] {}", format_tool_call(&tool_name, &tool_input));
                    let _ = event_tx.send(LogEvent::tool_call(tool_name, formatted) // Move tool_name
                        .with_tool_args(serde_json::json!({ "tool_use_id": tool_use_id })).for_job(job_id)).await;
                }
                BridgeEvent::Heartbeat { .. } => {}
            }
        }

        if !output_text.is_empty() { result.output_text = Some(output_text); }
        extract_output_from_result(&mut result.output_text, structured_result);
        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str { "claude" }
    fn is_available(&self) -> bool { self.client.health_check().is_ok() }
}
