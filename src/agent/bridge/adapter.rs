//! Bridge-based agent adapters.
//!
//! These adapters use the SDK Bridge for full SDK control instead of CLI invocation.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::sync::mpsc;

use super::client::BridgeClient;
use super::types::*;
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
    /// Create a new Claude bridge adapter
    pub fn new() -> Self {
        Self {
            client: BridgeClient::new(),
        }
    }

    /// Create with a custom bridge URL
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            client: BridgeClient::with_url(url),
        }
    }

    /// Build the prompt for a job
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");
        let ide_context = job.ide_context.as_deref().unwrap_or("");

        template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
            .replace("{ide_context}", ide_context)
    }

    /// Build system prompt
    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_mode_template(&job.mode);
        let mut system_prompt = template.system_prompt.unwrap_or_default();

        // Add worktree instruction if applicable
        if job.git_worktree_path.is_some() {
            system_prompt.push_str("\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.");
        }

        // Add output schema if configured
        if let Some(schema) = &config.output_schema {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(schema);
        }

        // Add output state instructions for chain workflows
        // Priority: 1) custom state_prompt, 2) auto-generate from output_states
        if let Some(ref state_prompt) = template.state_prompt {
            // Use custom state prompt (allows full control over wording)
            system_prompt.push_str("\n\n");
            system_prompt.push_str(state_prompt);
        } else if !template.output_states.is_empty() {
            // Auto-generate state instructions from output_states
            system_prompt.push_str("\n\n## Output State\n");
            system_prompt.push_str("When you complete this task, indicate the outcome by stating one of the following:\n");
            for state in &template.output_states {
                system_prompt.push_str(&format!("- state: {}\n", state));
            }
            system_prompt.push_str("\nExample: \"state: ");
            system_prompt.push_str(&template.output_states[0]);
            system_prompt.push_str("\" (at the end of your response)");
        }

        if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt)
        }
    }
}

impl Default for ClaudeBridgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for ClaudeBridgeAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config);
        let cwd = worktree.to_string_lossy().to_string();

        // Log start
        let _ = event_tx
            .send(
                LogEvent::system(format!("Starting Claude SDK job #{}", job_id)).for_job(job_id),
            )
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        // Build request
        let request = ClaudeQueryRequest {
            prompt: prompt.clone(),
            images: None,
            cwd,
            session_id: job.bridge_session_id.clone(),
            fork_session: None,
            permission_mode: Some(parse_claude_permission_mode(&config.permission_mode)),
            agents: if config.agents.is_empty() {
                None
            } else {
                Some(config.agents.clone())
            },
            allowed_tools: if config.allowed_tools.is_empty() {
                None
            } else {
                Some(config.allowed_tools.clone())
            },
            disallowed_tools: if config.disallowed_tools.is_empty() {
                None
            } else {
                Some(config.disallowed_tools.clone())
            },
            env: if config.env.is_empty() {
                None
            } else {
                Some(config.env.clone())
            },
            mcp_servers: if config.mcp_servers.is_empty() {
                None
            } else {
                Some(config.mcp_servers.clone())
            },
            system_prompt: self.build_system_prompt(job, config),
            system_prompt_mode: Some(match config.system_prompt_mode {
                crate::SystemPromptMode::Replace => "replace",
                crate::SystemPromptMode::Append | crate::SystemPromptMode::ConfigOverride => "append",
            }
            .to_string()),
            // Load Claude Code settings (incl. CLAUDE.md) for parity with the CLI.
            setting_sources: Some(vec![
                "user".to_string(),
                "project".to_string(),
                "local".to_string(),
            ]),
            plugins: {
                let plugins: Vec<ClaudePlugin> = config
                    .plugins
                    .iter()
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty())
                    .map(|path| ClaudePlugin {
                        plugin_type: ClaudePluginType::Local,
                        path: path.to_string(),
                    })
                    .collect();

                if plugins.is_empty() { None } else { Some(plugins) }
            },
            max_turns: if config.max_turns > 0 {
                Some(config.max_turns)
            } else {
                None
            },
            max_thinking_tokens: None,
            model: config.model.clone(),
            output_schema: parse_json_schema(config.structured_output_schema.as_deref()),
            kyco_callback_url: None,
            hooks: None,
        };

        let mut result = AgentResult {
            success: false,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            duration_ms: None,
            sent_prompt: Some(prompt),
            output_text: None,
            session_id: None,
        };

        let mut output_text = String::new();
        let mut captured_session_id: Option<String> = None;
        let mut structured_result: Option<serde_json::Value> = None;

        // Execute query and process events
        // Use a channel to stream events from the blocking task
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
        let client = self.client.clone();

        // Spawn blocking task to run the HTTP request and send events through channel
        tokio::task::spawn_blocking(move || {
            match client.claude_query(&request) {
                Ok(events) => {
                    for event_result in events {
                        let msg = event_result.map_err(|e| e.to_string());
                        if tx.blocking_send(msg).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e.to_string()));
                }
            }
        });

        // Process events from channel
        while let Some(event_result) = rx.recv().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    let _ = event_tx
                        .send(LogEvent::error(format!("Event parse error: {}", e)).for_job(job_id))
                        .await;
                    continue;
                }
            };

            match event {
                BridgeEvent::SessionStart {
                    session_id,
                    model,
                    tools,
                    ..
                } => {
                    // Capture session_id for continuation
                    captured_session_id = Some(session_id.clone());
                    let _ = event_tx
                        .send(
                            LogEvent::system(format!(
                                "Session started: {} tools available, model: {} (session: {})",
                                tools.len(),
                                model,
                                &session_id[..12.min(session_id.len())]
                            ))
                            .with_tool_args(serde_json::json!({ "session_id": session_id }))
                            .for_job(job_id),
                        )
                        .await;
                }

                BridgeEvent::Text {
                    content, partial, ..
                } => {
                    if !partial {
                        output_text.push_str(&content);
                        output_text.push('\n');
                    }
                    let _ = event_tx
                        .send(LogEvent::text(content).for_job(job_id))
                        .await;
                }

                BridgeEvent::ToolUse {
                    tool_name,
                    tool_input,
                    ..
                } => {
                    let summary = format_tool_call(&tool_name, &tool_input);
                    let _ = event_tx
                        .send(LogEvent::tool_call(tool_name, summary).for_job(job_id))
                        .await;
                }

                BridgeEvent::ToolResult {
                    success,
                    output,
                    files_changed,
                    ..
                } => {
                    if let Some(files) = files_changed {
                        for file in files {
                            result.changed_files.push(std::path::PathBuf::from(file));
                        }
                    }
                    let _ = event_tx
                        .send(
                            LogEvent::tool_output(
                                "tool",
                                if success {
                                    output
                                } else {
                                    format!("Error: {}", output)
                                },
                            )
                            .for_job(job_id),
                        )
                        .await;
                }

                BridgeEvent::Error { message, .. } => {
                    result.error = Some(message.clone());
                    let _ = event_tx
                        .send(LogEvent::error(message).for_job(job_id))
                        .await;
                }

                BridgeEvent::SessionComplete {
                    success,
                    cost_usd,
                    duration_ms,
                    usage,
                    result: session_result,
                    ..
                } => {
                    result.success = success;
                    result.cost_usd = cost_usd;
                    result.duration_ms = Some(duration_ms);
                    structured_result = session_result;

                    let usage_info = usage
                        .map(|u| {
                            format!(
                                ", {} input + {} output tokens",
                                u.input_tokens, u.output_tokens
                            )
                        })
                        .unwrap_or_default();

                    let _ = event_tx
                        .send(
                            LogEvent::system(format!(
                                "Completed: {} (cost: ${:.4}, duration: {}ms{})",
                                if success { "success" } else { "failed" },
                                cost_usd.unwrap_or(0.0),
                                duration_ms,
                                usage_info
                            ))
                            .for_job(job_id),
                        )
                        .await;
                }

                BridgeEvent::ToolApprovalNeeded {
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                    ..
                } => {
                    // Forward to GUI for user approval
                    // The executor converts this LogEvent into an ExecutorEvent::PermissionNeeded.
                    let _ = event_tx
                        .send(LogEvent::permission(format!("Tool approval needed: {}", tool_name))
                            .with_tool_args(serde_json::json!({
                                "request_id": request_id,
                                "session_id": session_id,
                                "tool_name": tool_name,
                                "tool_input": tool_input,
                            }))
                            .for_job(job_id))
                        .await;
                }

                BridgeEvent::HookPreToolUse {
                    tool_name,
                    tool_input,
                    tool_use_id,
                    ..
                } => {
                    let summary = format!("[hook PreToolUse] {}", format_tool_call(&tool_name, &tool_input));
                    let _ = event_tx
                        .send(
                            LogEvent::tool_call(tool_name, summary)
                                .with_tool_args(serde_json::json!({ "tool_use_id": tool_use_id }))
                                .for_job(job_id),
                        )
                        .await;
                }
            }
        }

        if !output_text.is_empty() {
            result.output_text = Some(output_text);
        }

        if result.output_text.is_none() {
            if let Some(value) = structured_result {
                if !value.is_null() {
                    match value {
                        serde_json::Value::String(s) => {
                            // If the structured result is itself a string, keep it as-is.
                            // Serializing it would add quotes and escape newlines (\"...\\n...\").
                            result.output_text = Some(s);
                        }
                        other => {
                            if let Ok(json) = serde_json::to_string_pretty(&other) {
                                result.output_text = Some(json);
                            }
                        }
                    }
                }
            }
        }

        // Set the session ID for continuation
        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str {
        "claude"
    }

    fn is_available(&self) -> bool {
        // Check if the bridge is running
        self.client.health_check().is_ok()
    }
}

/// Codex adapter using the SDK Bridge
pub struct CodexBridgeAdapter {
    client: BridgeClient,
}

impl CodexBridgeAdapter {
    /// Create a new Codex bridge adapter
    pub fn new() -> Self {
        Self {
            client: BridgeClient::new(),
        }
    }

    /// Create with a custom bridge URL
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            client: BridgeClient::with_url(url),
        }
    }

    /// Build the prompt for a job
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");
        let ide_context = job.ide_context.as_deref().unwrap_or("");

        let mut prompt = template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
            .replace("{ide_context}", ide_context);

        // For Codex we don't have a separate system prompt channel; append the YAML output schema
        // so the executor can reliably parse titles/summaries (used for commit messages, chains).
        if let Some(schema) = &config.output_schema {
            if !schema.trim().is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(schema);
            }
        }

        prompt
    }
}

impl Default for CodexBridgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for CodexBridgeAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config);
        let cwd = worktree.to_string_lossy().to_string();

        // Log start
        let _ = event_tx
            .send(LogEvent::system(format!("Starting Codex SDK job #{}", job_id)).for_job(job_id))
            .await;

        // Build request
        let request = CodexQueryRequest {
            prompt: prompt.clone(),
            images: None,
            cwd,
            thread_id: job.bridge_session_id.clone(),
            sandbox: config
                .sandbox
                .clone()
                .or_else(|| Some("workspace-write".to_string())),
            env: if config.env.is_empty() {
                None
            } else {
                Some(config.env.clone())
            },
            output_schema: parse_json_schema(config.structured_output_schema.as_deref()),
            model: None,
            effort: None,
            approval_policy: None,
            skip_git_repo_check: None,
        };

        let mut result = AgentResult {
            success: false,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            duration_ms: None,
            sent_prompt: Some(prompt),
            output_text: None,
            session_id: None, // Filled from session.start (thread id)
        };

        let mut output_text = String::new();
        let mut captured_session_id: Option<String> = None;
        let mut structured_result: Option<serde_json::Value> = None;

        // Execute query using channel to stream events from blocking task
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
        let client = self.client.clone();

        tokio::task::spawn_blocking(move || {
            match client.codex_query(&request) {
                Ok(events) => {
                    for event_result in events {
                        let msg = event_result.map_err(|e| e.to_string());
                        if tx.blocking_send(msg).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e.to_string()));
                }
            }
        });

        while let Some(event_result) = rx.recv().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    let _ = event_tx
                        .send(LogEvent::error(format!("Event error: {}", e)).for_job(job_id))
                        .await;
                    continue;
                }
            };

            match event {
                BridgeEvent::SessionStart { session_id, .. } => {
                    captured_session_id = Some(session_id.clone());
                    let _ = event_tx
                        .send(
                            LogEvent::system("Codex thread started")
                                .with_tool_args(serde_json::json!({ "session_id": session_id }))
                                .for_job(job_id),
                        )
                        .await;
                }

                BridgeEvent::Text {
                    content, partial, ..
                } => {
                    if !partial {
                        output_text.push_str(&content);
                        output_text.push('\n');
                    }
                    let _ = event_tx
                        .send(LogEvent::text(content).for_job(job_id))
                        .await;
                }

                BridgeEvent::ToolUse {
                    tool_name,
                    tool_input,
                    ..
                } => {
                    let summary = format_tool_call(&tool_name, &tool_input);
                    let _ = event_tx
                        .send(LogEvent::tool_call(tool_name, summary).for_job(job_id))
                        .await;
                }

                BridgeEvent::ToolResult {
                    output,
                    files_changed,
                    ..
                } => {
                    if let Some(files) = files_changed {
                        for file in files {
                            result.changed_files.push(std::path::PathBuf::from(file));
                        }
                    }
                    let _ = event_tx
                        .send(LogEvent::tool_output("tool", output).for_job(job_id))
                        .await;
                }

                BridgeEvent::Error { message, .. } => {
                    result.error = Some(message.clone());
                    let _ = event_tx
                        .send(LogEvent::error(message).for_job(job_id))
                        .await;
                }

                BridgeEvent::SessionComplete {
                    success,
                    duration_ms,
                    usage,
                    result: session_result,
                    ..
                } => {
                    result.success = success;
                    result.duration_ms = Some(duration_ms);
                    structured_result = session_result;

                    let usage_info = usage
                        .map(|u| format!(", {} tokens", u.input_tokens + u.output_tokens))
                        .unwrap_or_default();

                    let _ = event_tx
                        .send(
                            LogEvent::system(format!(
                                "Completed: {} (duration: {}ms{})",
                                if success { "success" } else { "failed" },
                                duration_ms,
                                usage_info
                            ))
                            .for_job(job_id),
                        )
                        .await;
                }

                _ => {}
            }
        }

        if !output_text.is_empty() {
            result.output_text = Some(output_text);
        }

        if result.output_text.is_none() {
            if let Some(value) = structured_result {
                if !value.is_null() {
                    match value {
                        serde_json::Value::String(s) => {
                            // If the structured result is itself a string, keep it as-is.
                            // Serializing it would add quotes and escape newlines (\"...\\n...\").
                            result.output_text = Some(s);
                        }
                        other => {
                            if let Ok(json) = serde_json::to_string_pretty(&other) {
                                result.output_text = Some(json);
                            }
                        }
                    }
                }
            }
        }

        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str {
        "codex"
    }

    fn is_available(&self) -> bool {
        self.client.health_check().is_ok()
    }
}

/// Format a tool call for display
fn format_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" | "read" => input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .map(|p| format!("Read {}", p))
            .unwrap_or_else(|| "Read file".to_string()),

        "Write" | "write" => input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .map(|p| format!("Write {}", p))
            .unwrap_or_else(|| "Write file".to_string()),

        "Edit" | "edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|p| format!("Edit {}", p))
            .unwrap_or_else(|| "Edit file".to_string()),

        "Bash" | "bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|c| format!("Bash: {}", c))
            .unwrap_or_else(|| "Bash command".to_string()),

        _ => name.to_string(),
    }
}

fn parse_claude_permission_mode(mode: &str) -> PermissionMode {
    match mode {
        "default" => PermissionMode::Default,
        "acceptEdits" | "accept_edits" | "accept-edits" => PermissionMode::AcceptEdits,
        "bypassPermissions" | "bypass_permissions" | "bypass-permissions" => {
            PermissionMode::BypassPermissions
        }
        "plan" => PermissionMode::Plan,
        _ => PermissionMode::Default,
    }
}

fn parse_json_schema(schema: Option<&str>) -> Option<serde_json::Value> {
    let schema = schema?.trim();
    if !(schema.starts_with('{') || schema.starts_with('[')) {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(schema).ok()?;
    if value.is_object() {
        Some(value)
    } else {
        None
    }
}
