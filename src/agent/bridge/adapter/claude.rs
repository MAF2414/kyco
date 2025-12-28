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

    fn build_request(&self, job: &Job, config: &AgentConfig, prompt: String, cwd: String) -> ClaudeQueryRequest {
        ClaudeQueryRequest {
            prompt,
            images: None,
            cwd,
            session_id: job.bridge_session_id.clone(),
            fork_session: None,
            permission_mode: Some(parse_claude_permission_mode(&config.permission_mode)),
            agents: if config.agents.is_empty() { None } else { Some(config.agents.clone()) },
            allowed_tools: if config.allowed_tools.is_empty() { None } else { Some(config.allowed_tools.clone()) },
            disallowed_tools: if config.disallowed_tools.is_empty() { None } else { Some(config.disallowed_tools.clone()) },
            env: if config.env.is_empty() { None } else { Some(config.env.clone()) },
            mcp_servers: if config.mcp_servers.is_empty() { None } else { Some(config.mcp_servers.clone()) },
            system_prompt: self.build_system_prompt(job, config),
            system_prompt_mode: Some(match config.system_prompt_mode {
                crate::SystemPromptMode::Replace => "replace",
                crate::SystemPromptMode::Append | crate::SystemPromptMode::ConfigOverride => "append",
            }.to_string()),
            setting_sources: Some(vec!["user".to_string(), "project".to_string(), "local".to_string()]),
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

        let request = self.build_request(job, config, prompt.clone(), cwd);
        let mut result = AgentResult {
            success: false, error: None, changed_files: Vec::new(), cost_usd: None,
            duration_ms: None, sent_prompt: Some(prompt), output_text: None, session_id: None,
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
                BridgeEvent::SessionStart { session_id, model, tools, .. } => {
                    captured_session_id = Some(session_id.clone());
                    let preview = session_id.get(..12).unwrap_or(&session_id);
                    let _ = event_tx.send(LogEvent::system(format!("Session started: {} tools available, model: {} (session: {})", tools.len(), model, preview))
                        .with_tool_args(serde_json::json!({ "session_id": session_id })).for_job(job_id)).await;
                }
                BridgeEvent::Text { content, partial, .. } => {
                    if !partial { output_text.push_str(&content); output_text.push('\n'); }
                    let _ = event_tx.send(LogEvent::text(content).for_job(job_id)).await;
                }
                BridgeEvent::ToolUse { tool_name, tool_input, .. } => {
                    let _ = event_tx.send(LogEvent::tool_call(tool_name.clone(), format_tool_call(&tool_name, &tool_input)).for_job(job_id)).await;
                }
                BridgeEvent::ToolResult { success, output, files_changed, .. } => {
                    if let Some(files) = files_changed { for f in files { result.changed_files.push(std::path::PathBuf::from(f)); } }
                    let _ = event_tx.send(LogEvent::tool_output("tool", if success { output } else { format!("Error: {}", output) }).for_job(job_id)).await;
                }
                BridgeEvent::Error { message, .. } => {
                    result.error = Some(message.clone());
                    let _ = event_tx.send(LogEvent::error(message).for_job(job_id)).await;
                }
                BridgeEvent::SessionComplete { success, cost_usd, duration_ms, usage, result: sr, .. } => {
                    result.success = success; result.cost_usd = cost_usd; result.duration_ms = Some(duration_ms); structured_result = sr;
                    let usage_info = usage.map(|u| format!(", {} input + {} output tokens", u.input_tokens, u.output_tokens)).unwrap_or_default();
                    let _ = event_tx.send(LogEvent::system(format!("Completed: {} (cost: ${:.4}, duration: {}ms{})", if success { "success" } else { "failed" }, cost_usd.unwrap_or(0.0), duration_ms, usage_info)).for_job(job_id)).await;
                }
                BridgeEvent::ToolApprovalNeeded { request_id, session_id, tool_name, tool_input, .. } => {
                    let _ = event_tx.send(LogEvent::permission(format!("Tool approval needed: {}", tool_name))
                        .with_tool_args(serde_json::json!({ "request_id": request_id, "session_id": session_id, "tool_name": tool_name, "tool_input": tool_input })).for_job(job_id)).await;
                }
                BridgeEvent::HookPreToolUse { tool_name, tool_input, tool_use_id, .. } => {
                    let _ = event_tx.send(LogEvent::tool_call(tool_name.clone(), format!("[hook PreToolUse] {}", format_tool_call(&tool_name, &tool_input)))
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
