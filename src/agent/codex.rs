//! Codex CLI agent adapter

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

/// Codex CLI agent adapter
///
/// Codex CLI uses `codex exec --json "prompt"` for non-interactive mode.
/// Output format differs from Claude Code.
pub struct CodexAdapter {
    id: String,
}

impl CodexAdapter {
    /// Create a new Codex adapter
    pub fn new() -> Self {
        Self {
            id: "codex".to_string(),
        }
    }

    /// Build the prompt for a job using the mode template from config
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");

        // Replace template placeholders
        template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
    }

    /// Build command arguments for Codex CLI
    ///
    /// Codex CLI format: `codex exec [OPTIONS] PROMPT`
    /// Key options:
    /// - `--json` for JSONL output
    /// - `--full-auto` for unattended work (workspace-write sandbox, approvals on failure)
    /// - `--yolo` for bypassing all approvals and sandbox
    /// - `-C PATH` for working directory
    fn build_args(&self, job: &Job, config: &AgentConfig, prompt: &str) -> Vec<String> {
        let mut args = config.get_run_args();

        // Build the full prompt with system prompt if configured
        let template = config.get_mode_template(&job.mode);
        let mut system_prompt = template.system_prompt.clone().unwrap_or_default();

        // If running in a worktree, add commit instruction
        if job.git_worktree_path.is_some() {
            let commit_instruction = "\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.";
            system_prompt.push_str(commit_instruction);
        }

        let full_prompt = if !system_prompt.is_empty() {
            format!("{}\n\n{}", system_prompt, prompt)
        } else {
            prompt.to_string()
        };

        // Add disallowed tools if configured (Codex doesn't support this directly,
        // but we could add it to the prompt as instructions)
        // For now, skip this - Codex handles permissions differently

        // Add -- separator to indicate end of flags, then the prompt
        args.push("--".to_string());
        args.push(full_prompt);

        args
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for CodexAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let prompt = self.build_prompt(job, config);
        let args = self.build_args(job, config, &prompt);

        // Send start event with full prompt
        let job_id = job.id;
        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting job #{} with prompt:",
                job_id
            )).for_job(job_id))
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        // Spawn the process
        let mut child = Command::new(&config.binary)
            .args(&args)
            .current_dir(worktree)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env)
            .spawn()
            .with_context(|| format!("Failed to spawn {}", config.binary))?;

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");
        let mut reader = BufReader::new(stdout).lines();

        // Spawn a task to read stderr
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
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
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
        };

        // Track if we received turn.completed (means success regardless of exit code)
        let mut turn_completed = false;

        // Process output stream
        while let Ok(Some(line)) = reader.next_line().await {
            match parse_codex_event(&line) {
                CodexEventResult::Log(event) => {
                    // Check if this is the completion message
                    if event.summary.starts_with("Completed (tokens:") {
                        turn_completed = true;
                    }
                    let _ = event_tx.send(event.for_job(job_id)).await;
                }
                CodexEventResult::None => {}
            }
        }

        // Wait for the process to finish
        let status = child.wait().await?;

        // Success is based on turn.completed, not exit code
        // Codex may exit with code 1 even on success (e.g., if tests fail)
        if turn_completed {
            result.success = true;
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
                .await;
        } else if status.success() {
            result.success = true;
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
                .await;
        } else {
            result.error = Some(format!("Process exited with status: {}", status));
            let _ = event_tx
                .send(LogEvent::error(format!("Job #{} failed: {}", job_id, status)).for_job(job_id))
                .await;
        }

        Ok(result)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg("codex")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Result from parsing a Codex event
enum CodexEventResult {
    /// A log event to display
    Log(LogEvent),
    /// No event to display
    None,
}

/// Parse a Codex JSON output line
///
/// Codex exec --json output format:
/// - `item.started` / `item.completed` - individual steps
/// - `turn.completed` - task finished (with usage stats)
/// - `message` - assistant messages
/// - `error` - errors
fn parse_codex_event(line: &str) -> CodexEventResult {
    let json: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return CodexEventResult::None,
    };

    let event_type = match json.get("type").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return CodexEventResult::None,
    };

    match event_type {
        // Turn completed = success
        "turn.completed" => {
            // Extract usage info if available
            let usage = json.get("usage");
            let input_tokens = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0);

            CodexEventResult::Log(LogEvent::system(format!(
                "Completed (tokens: {} in, {} out)",
                input_tokens, output_tokens
            )))
        }

        // Item events - show reasoning and commands
        "item.completed" | "item.started" => {
            let item = match json.get("item") {
                Some(i) => i,
                None => return CodexEventResult::None,
            };

            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");

            match item_type {
                "reasoning" => {
                    let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    // Only show first line of reasoning
                    let first_line = text.lines().next().unwrap_or("");
                    CodexEventResult::Log(LogEvent::thought(truncate(first_line, 100)))
                }
                "command_execution" => {
                    let cmd = item.get("command").and_then(|c| c.as_str()).unwrap_or("");
                    if event_type == "item.started" {
                        CodexEventResult::Log(LogEvent::tool_call("bash", truncate(cmd, 80)))
                    } else {
                        // For completed, we could show output but it's often long
                        CodexEventResult::None
                    }
                }
                "agent_message" => {
                    let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    let first_line = text.lines().next().unwrap_or("");
                    CodexEventResult::Log(LogEvent::text(truncate(first_line, 150)))
                }
                "file_edit" | "file_create" => {
                    let path = item.get("path").and_then(|p| p.as_str()).unwrap_or("file");
                    CodexEventResult::Log(LogEvent::tool_call(item_type, path.to_string()))
                }
                _ => CodexEventResult::None,
            }
        }

        // Legacy message format
        "message" => {
            let content = match json.get("content").and_then(|c| c.as_str()) {
                Some(c) => c,
                None => return CodexEventResult::None,
            };
            let role = json.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");

            if role == "assistant" {
                CodexEventResult::Log(LogEvent::text(truncate(content, 200)))
            } else {
                CodexEventResult::Log(LogEvent::system(format!("[{}] {}", role, truncate(content, 100))))
            }
        }

        // Error events
        "error" => {
            let message = json.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            CodexEventResult::Log(LogEvent::error(message.to_string()))
        }

        // Ignore other event types silently
        "session.created" | "session.updated" | "item.input_audio_transcription.completed" => {
            CodexEventResult::None
        }

        _ => CodexEventResult::None,
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}
