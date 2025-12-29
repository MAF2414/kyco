//! Codex CLI agent adapter

mod parser;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};
use parser::{CodexEventResult, parse_codex_event};

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

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture stdout pipe")?;
        let stderr = child
            .stderr
            .take()
            .context("Failed to capture stderr pipe")?;
        let mut reader = BufReader::new(stdout).lines();

        let event_tx_clone = event_tx.clone();
        let stderr_task = tokio::spawn(async move {
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
            session_id: None, // CLI adapter doesn't support session continuation
        };

        // Track if we received turn.completed (means success regardless of exit code)
        let mut turn_completed = false;

        while let Ok(Some(line)) = reader.next_line().await {
            match parse_codex_event(&line) {
                CodexEventResult::Log(event) => {
                    if event.summary.starts_with("Completed (tokens:") {
                        turn_completed = true;
                    }
                    let _ = event_tx.send(event.for_job(job_id)).await;
                }
                CodexEventResult::None => {}
            }
        }

        let status = child.wait().await?;

        // Wait for stderr task to complete to ensure all logs are captured
        // before sending completion message (prevents race condition in log ordering)
        let _ = stderr_task.await;

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
                .send(
                    LogEvent::error(format!("Job #{} failed: {}", job_id, status)).for_job(job_id),
                )
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
