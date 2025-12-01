//! Gemini CLI agent adapter

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

use super::parser::parse_gemini_event;

/// Gemini CLI agent adapter
///
/// Gemini CLI uses a different invocation pattern and relies on GEMINI.md files
/// for system prompts. The CLI may not have a standard non-interactive mode
/// documented, so this adapter provides a best-effort implementation.
pub struct GeminiAdapter {
    id: String,
}

impl GeminiAdapter {
    /// Create a new Gemini adapter
    pub fn new() -> Self {
        Self {
            id: "gemini".to_string(),
        }
    }

    /// Build the prompt for a job
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);

        let description = job.description.as_deref().unwrap_or("");

        template
            .prompt_template
            .replace("{target}", &job.target)
            .replace("{file}", &job.source_file.display().to_string())
            .replace("{line}", &job.source_line.to_string())
            .replace("{description}", description)
            .replace("{mode}", &job.mode)
    }

    /// Write system prompt to GEMINI.md if needed
    async fn setup_system_prompt(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
    ) -> Result<Option<std::path::PathBuf>> {
        let template = config.get_mode_template(&job.mode);
        let mut system_prompt = template.system_prompt.clone().unwrap_or_default();

        // If running in a worktree, add commit instruction
        if job.git_worktree_path.is_some() {
            let commit_instruction = "\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.";
            system_prompt.push_str(commit_instruction);
        }

        if !system_prompt.is_empty() {
            let gemini_md_path = worktree.join("GEMINI.md");

            // Read existing GEMINI.md if it exists
            let existing_content = tokio::fs::read_to_string(&gemini_md_path)
                .await
                .unwrap_or_default();

            // Append KYCo section if not already present
            let kyco_section = format!(
                "\n\n## KYCo Mode: {}\n\n{}\n",
                job.mode, system_prompt
            );

            if !existing_content.contains("## KYCo Mode:") {
                let new_content = format!("{}{}", existing_content, kyco_section);
                tokio::fs::write(&gemini_md_path, new_content).await?;
                return Ok(Some(gemini_md_path));
            }
        }

        Ok(None)
    }

    /// Build command arguments for Gemini CLI
    fn build_args(&self, config: &AgentConfig, prompt: &str) -> Vec<String> {
        let mut args = config.get_run_args();

        // Add the prompt
        // Gemini CLI might use different argument patterns
        // This is a best-effort implementation
        args.push(prompt.to_string());

        args
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for GeminiAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let prompt = self.build_prompt(job, config);

        // Setup GEMINI.md for system prompt
        let _gemini_md = self.setup_system_prompt(job, worktree, config).await?;

        let args = self.build_args(config, &prompt);

        // Send start event
        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting gemini for job #{}",
                job.id
            )))
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
        let mut reader = BufReader::new(stdout).lines();

        let mut result = AgentResult {
            success: false,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
            output_text: None,
        };

        // Process output stream
        // Gemini output format is not well-documented for non-interactive mode
        // We'll try to parse JSON if available, otherwise treat as text
        while let Ok(Some(line)) = reader.next_line().await {
            let event = parse_gemini_event(&line);
            let _ = event_tx.send(event).await;
        }

        // Wait for the process to finish
        let status = child.wait().await?;

        if status.success() {
            result.success = true;
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job.id)))
                .await;
        } else {
            result.error = Some(format!("Process exited with status: {}", status));
            let _ = event_tx
                .send(LogEvent::error(format!("Job #{} failed: {}", job.id, status)))
                .await;
        }

        Ok(result)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg("gemini")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
