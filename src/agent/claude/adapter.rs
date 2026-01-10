//! Claude Code agent adapter

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::output::{ContentBlock, StreamEvent};
use super::tool_format::format_tool_call;
use crate::agent::process_registry;
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

pub struct ClaudeAdapter {
    id: String,
}

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self {
            id: "claude".to_string(),
        }
    }

    /// Build the prompt for a job using the mode template from config
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_skill_template(&job.skill);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");

        let ide_context = job.ide_context.as_deref().unwrap_or("");
        template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.skill)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
            .replace("{ide_context}", ide_context)
    }

    /// Build the system prompt addition for a job
    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_skill_template(&job.skill);
        let mut system_prompt = template.system_prompt.unwrap_or_default();

        // If running in a worktree, add commit instruction
        if job.git_worktree_path.is_some() {
            let commit_instruction = "\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.";
            system_prompt.push_str(commit_instruction);
        }

        // Append output schema if configured
        if let Some(schema) = &config.output_schema {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(schema);
        }

        if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt)
        }
    }

    /// Build command arguments
    fn build_args(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        prompt: &str,
    ) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(session_id) = job.bridge_session_id.as_deref() {
            args.push("--resume".to_string());
            args.push(session_id.to_string());
        }

        if let Some(model) = config.model.as_deref() {
            args.push("--model".to_string());
            args.push(model.to_string());
        }

        if config.allow_dangerous_bypass {
            args.push("--dangerously-skip-permissions".to_string());
        }

        if !config.permission_mode.trim().is_empty() {
            args.push("--permission-mode".to_string());
            args.push(config.permission_mode.clone());
        }

        // Add system prompt if configured
        if let Some(system_prompt) = self.build_system_prompt(job, config) {
            match config.system_prompt_mode {
                crate::SystemPromptMode::Append => {
                    args.push("--append-system-prompt".to_string());
                    args.push(system_prompt);
                }
                crate::SystemPromptMode::Replace => {
                    args.push("--system-prompt".to_string());
                    args.push(system_prompt);
                }
                crate::SystemPromptMode::ConfigOverride => {
                    // ConfigOverride not applicable for Claude - treat as append
                    args.push("--append-system-prompt".to_string());
                    args.push(system_prompt);
                }
            }
        }

        // Add disallowed tools (each tool as a separate argument)
        if !config.disallowed_tools.is_empty() {
            args.push("--disallowedTools".to_string());
            for tool in &config.disallowed_tools {
                args.push(tool.clone());
            }
        }

        // Add allowed tools (each tool as a separate argument)
        if !config.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            for tool in &config.allowed_tools {
                args.push(tool.clone());
            }
        }

        if let Some(add_dir) = find_skill_add_dir(job, worktree) {
            args.push("--add-dir".to_string());
            args.push(add_dir.display().to_string());
        }

        // Claude's CLI has several variadic options (e.g. --add-dir/--allowedTools) that will
        // greedily consume trailing args unless we explicitly end option parsing.
        args.push("--".to_string());
        args.push(prompt.to_string());

        args
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for ClaudeAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        struct ProcessGuard {
            job_id: u64,
            registered: bool,
        }
        impl ProcessGuard {
            fn register(job_id: u64, pid: Option<u32>, agent_id: &str) -> Self {
                if let Some(pid) = pid {
                    process_registry::register(job_id, pid, agent_id);
                    return Self {
                        job_id,
                        registered: true,
                    };
                }
                Self {
                    job_id,
                    registered: false,
                }
            }
        }
        impl Drop for ProcessGuard {
            fn drop(&mut self) {
                if self.registered {
                    process_registry::unregister(self.job_id);
                }
            }
        }

        let prompt = self.build_prompt(job, config);
        let args = self.build_args(job, worktree, config, &prompt);

        // Send start event with full prompt
        let job_id = job.id;
        let _ = event_tx
            .send(
                LogEvent::system(format!("Starting job #{} with prompt:", job_id)).for_job(job_id),
            )
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        let binary = config.get_binary();
        let mut child = Command::new(&binary)
            .args(&args)
            .current_dir(worktree)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env)
            .spawn()
            .with_context(|| format!("Failed to spawn {}", binary))?;

        let _process_guard = ProcessGuard::register(job_id, child.id(), self.id());

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture stdout pipe")?;
        let stderr = child
            .stderr
            .take()
            .context("Failed to capture stderr pipe")?;
        let mut reader = BufReader::new(stdout).lines();

        // Spawn a task to read stderr - keep handle to await later
        let event_tx_clone = event_tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                let _ = event_tx_clone
                    .send(LogEvent::error(format!("stderr: {}", line)).for_job(job_id))
                    .await;
            }
        });

        let mut result = AgentResult {
            success: false,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            cache_read_tokens: None,
            cache_write_tokens: None,
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
            output_text: None,
            session_id: job.bridge_session_id.clone(),
        };

        // Collect text output for parsing
        let mut output_text = String::new();

        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(event) = StreamEvent::parse(&line) {
                let log_event = match &event {
                    StreamEvent::System {
                        subtype,
                        message,
                        session_id,
                    } => {
                        if let Some(id) = session_id.as_deref() {
                            if !id.is_empty() && result.session_id.is_none() {
                                result.session_id = Some(id.to_string());
                                let _ = event_tx
                                    .send(
                                        LogEvent::system("Claude session started")
                                            .with_tool_args(serde_json::json!({ "session_id": id }))
                                            .for_job(job_id),
                                    )
                                    .await;
                            }
                        }
                        LogEvent::system(format!(
                            "{}: {}",
                            subtype,
                            message.as_deref().unwrap_or("")
                        ))
                    }
                    StreamEvent::Assistant { message } => {
                        let mut events = Vec::new();
                        for block in &message.content {
                            match block {
                                ContentBlock::Text { text } => {
                                    // Collect text output for parsing ---kyco blocks
                                    output_text.push_str(text);
                                    output_text.push('\n');
                                    events.push(LogEvent::text(text.clone()));
                                }
                                ContentBlock::ToolUse { name, input, .. } => {
                                    let summary = format_tool_call(name, input);
                                    events.push(LogEvent::tool_call(name.clone(), summary));
                                }
                                _ => {}
                            }
                        }
                        // Send all but the last, return the last
                        for evt in events.drain(..events.len().saturating_sub(1)) {
                            let _ = event_tx.send(evt.for_job(job_id)).await;
                        }
                        events
                            .pop()
                            .unwrap_or_else(|| LogEvent::system("assistant message"))
                    }
                    StreamEvent::User { message } => {
                        let mut summary = String::new();
                        for block in &message.content {
                            if let ContentBlock::ToolResult {
                                content, is_error, ..
                            } = block
                            {
                                summary = if *is_error {
                                    format!("Error: {}", content)
                                } else {
                                    content.clone()
                                };
                            }
                        }
                        LogEvent::tool_output("tool", summary)
                    }
                    StreamEvent::Result {
                        subtype,
                        cost_usd,
                        duration_ms,
                        session_id,
                        ..
                    } => {
                        result.cost_usd = *cost_usd;
                        result.duration_ms = *duration_ms;
                        if let Some(id) = session_id.as_deref() {
                            if !id.is_empty() && result.session_id.is_none() {
                                result.session_id = Some(id.to_string());
                            }
                        }

                        if subtype == "success" {
                            result.success = true;
                        }

                        LogEvent::system(format!(
                            "Completed: {} (cost: ${:.4}, duration: {}ms)",
                            subtype,
                            cost_usd.unwrap_or(0.0),
                            duration_ms.unwrap_or(0)
                        ))
                    }
                };

                let _ = event_tx.send(log_event.for_job(job_id)).await;
            }
        }

        let status = child.wait().await?;

        // Wait for stderr reader to finish processing all output
        // Ignore errors from the task itself (e.g., if it panicked)
        let _ = stderr_handle.await;

        if !status.success() && !result.success {
            result.error = Some(format!("Process exited with status: {}", status));
        }

        if !output_text.is_empty() {
            result.output_text = Some(output_text);
        }

        Ok(result)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg("claude")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

fn find_skill_add_dir(job: &Job, worktree: &Path) -> Option<std::path::PathBuf> {
    let skill = job.skill.as_str();
    let mut candidates = Vec::new();

    candidates.push(worktree.join(".claude/skills").join(skill));
    candidates.push(worktree.join(".claude/skills"));

    if let Some(workspace) = job.workspace_path.as_ref() {
        candidates.push(workspace.join(".claude/skills").join(skill));
        candidates.push(workspace.join(".claude/skills"));
    }

    if let Some(worktree) = job.git_worktree_path.as_ref() {
        candidates.push(worktree.join(".claude/skills").join(skill));
        candidates.push(worktree.join(".claude/skills"));
    }

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".claude/skills").join(skill));
        candidates.push(home.join(".claude/skills"));
        candidates.push(home.join(".kyco/skills").join(skill));
        candidates.push(home.join(".kyco/skills"));
    }

    for path in candidates {
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}
#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
