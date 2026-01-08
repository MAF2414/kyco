//! Terminal adapter implementing AgentRunner for REPL mode.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use super::session::{register_session, unregister_session, TerminalSession};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, CliType, Job, LogEvent, SystemPromptMode};

/// Terminal-based agent adapter for REPL mode.
///
/// Implements the [`AgentRunner`] trait to spawn AI agents in a separate
/// Terminal.app window. This adapter is designed for interactive REPL sessions
/// where users want to see agent output in real-time and potentially interact.
///
/// # Supported CLIs
///
/// - **Claude** (`claude`): Anthropic's Claude Code CLI
/// - **Codex** (`codex`): OpenAI's Codex CLI
///
/// # Behavior
///
/// When [`run`](AgentRunner::run) is called:
/// 1. Builds the prompt from job + config templates
/// 2. Spawns a [`TerminalSession`] with the agent command
/// 3. Registers the session for focus commands
/// 4. Polls until the agent process exits
/// 5. Returns an [`AgentResult`] (success assumed on completion)
pub struct TerminalAdapter {
    /// Unique identifier for this adapter instance
    id: String,
    /// The CLI type this adapter manages
    cli_type: CliType,
}

impl TerminalAdapter {
    /// Create a new terminal adapter for a given CLI type.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this adapter (e.g., "claude-terminal")
    /// * `cli_type` - The CLI variant to use
    pub fn new(id: impl Into<String>, cli_type: CliType) -> Self {
        Self {
            id: id.into(),
            cli_type,
        }
    }

    /// Create a terminal adapter for Claude Code CLI.
    ///
    /// Uses the identifier "claude-terminal".
    pub fn claude() -> Self {
        Self::new("claude-terminal", CliType::Claude)
    }

    /// Create a terminal adapter for OpenAI Codex CLI.
    ///
    /// Uses the identifier "codex-terminal".
    pub fn codex() -> Self {
        Self::new("codex-terminal", CliType::Codex)
    }

    /// Build the prompt for a job using the mode template from config.
    ///
    /// Substitutes template placeholders (`{file}`, `{line}`, `{target}`, etc.)
    /// with actual job values.
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_skill_template(&job.mode);
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

    /// Build the system prompt for a job.
    ///
    /// Returns the mode's system prompt with any worktree-specific instructions
    /// appended. Returns `None` if no system prompt is configured.
    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_skill_template(&job.mode);
        let mut system_prompt = template.system_prompt.unwrap_or_default();

        // If running in a worktree, add commit instruction
        if job.git_worktree_path.is_some() {
            let commit_instruction = "\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.";
            system_prompt.push_str(commit_instruction);
        }

        if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt)
        }
    }

    /// Build command arguments including system prompt for REPL mode.
    ///
    /// Constructs the CLI arguments based on the agent type and configuration:
    /// - Adds system prompt flags (`--append-system-prompt` or `--system-prompt`)
    /// - Adds tool restrictions (`--disallowedTools`, `--allowedTools`)
    fn build_repl_args(&self, job: &Job, config: &AgentConfig) -> Vec<String> {
        let mut args = config.get_repl_args();

        // Add system prompt if configured
        if let Some(system_prompt) = self.build_system_prompt(job, config) {
            match config.system_prompt_mode {
                SystemPromptMode::Append => {
                    if self.cli_type == CliType::Claude {
                        args.push("--append-system-prompt".to_string());
                        args.push(system_prompt);
                    }
                }
                SystemPromptMode::Replace => {
                    if self.cli_type == CliType::Claude {
                        args.push("--system-prompt".to_string());
                        args.push(system_prompt);
                    }
                }
                SystemPromptMode::ConfigOverride => {
                    if self.cli_type == CliType::Claude {
                        args.push("--append-system-prompt".to_string());
                        args.push(system_prompt);
                    }
                }
            }
        }

        // Add disallowed tools if any
        if !config.disallowed_tools.is_empty() && self.cli_type == CliType::Claude {
            args.push("--disallowedTools".to_string());
            for tool in &config.disallowed_tools {
                args.push(tool.clone());
            }
        }

        // Add allowed tools if any
        if !config.allowed_tools.is_empty() && self.cli_type == CliType::Claude {
            args.push("--allowedTools".to_string());
            for tool in &config.allowed_tools {
                args.push(tool.clone());
            }
        }

        args
    }
}

#[async_trait]
impl AgentRunner for TerminalAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config);

        let _ = event_tx
            .send(LogEvent::system(format!("Starting terminal job #{}", job_id)).for_job(job_id))
            .await;

        let repl_args = self.build_repl_args(job, config);
        let binary = config.get_binary();
        let session = TerminalSession::spawn(job_id, &binary, &repl_args, &prompt, worktree)?;

        let session = Arc::new(session);
        register_session(session.clone());

        let _ = event_tx
            .send(
                LogEvent::system(format!(
                    "Job #{} running in Terminal.app (press 'f' to focus)",
                    job_id
                ))
                .for_job(job_id),
            )
            .await;

        // Clone for the background task
        let session_clone = session.clone();
        let event_tx_clone = event_tx.clone();

        // Spawn a background task to poll for completion
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;

                if !session_clone.is_running() {
                    let _ = event_tx_clone
                        .send(
                            LogEvent::system(format!("Job #{} terminal session ended", job_id))
                                .for_job(job_id),
                        )
                        .await;
                    break;
                }
            }
        });

        handle.await?;
        unregister_session(job_id);

        let _ = event_tx
            .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
            .await;

        Ok(AgentResult {
            success: true,
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
            session_id: None, // Terminal mode doesn't support session continuation
        })
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            let binary = match self.cli_type {
                CliType::Claude => "claude",
                CliType::Codex => "codex",
                CliType::Gemini | CliType::Custom => return false,
            };

            std::process::Command::new("which")
                .arg(binary)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "macos"))]
        {
            false // Terminal adapter only available on macOS for now
        }
    }
}
