//! PTY-based agent adapter for REPL mode
//!
//! Runs agents in a pseudo-terminal for interactive sessions.
//! This allows full terminal emulation and streaming output to the TUI.

use anyhow::{Context, Result};
use async_trait::async_trait;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, CliType, Job, LogEvent};

/// PTY-based agent adapter for REPL mode
pub struct PtyAdapter {
    id: String,
    cli_type: CliType,
}

impl PtyAdapter {
    /// Create a new PTY adapter for a given CLI type
    pub fn new(id: impl Into<String>, cli_type: CliType) -> Self {
        Self {
            id: id.into(),
            cli_type,
        }
    }

    /// Create a PTY adapter for Claude
    pub fn claude() -> Self {
        Self::new("claude-repl", CliType::Claude)
    }

    /// Create a PTY adapter for Codex
    pub fn codex() -> Self {
        Self::new("codex-repl", CliType::Codex)
    }

    /// Create a PTY adapter for Gemini
    pub fn gemini() -> Self {
        Self::new("gemini-repl", CliType::Gemini)
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

    /// Build the system prompt for a job
    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_mode_template(&job.mode);
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
}

#[async_trait]
impl AgentRunner for PtyAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config);

        // Log start
        let _ = event_tx
            .send(LogEvent::system(format!("Starting REPL job #{}", job_id)).for_job(job_id))
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        // Create PTY
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        // Build command
        let mut cmd = CommandBuilder::new(&config.binary);
        cmd.args(config.get_repl_args());

        // Add system prompt if available (for Claude)
        if let Some(system_prompt) = self.build_system_prompt(job, config) {
            if self.cli_type == CliType::Claude {
                cmd.arg("--append-system-prompt");
                cmd.arg(&system_prompt);
            }
            // For Codex/Gemini, system prompt would need different handling
        }

        // For Claude/Codex in REPL mode, we pass the prompt as argument
        // For truly interactive mode, we'd write to stdin after launch
        cmd.arg("--");
        cmd.arg(&prompt);

        cmd.cwd(worktree);
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn child in PTY
        let mut child = pair.slave.spawn_command(cmd).context("Failed to spawn agent in PTY")?;

        // Get reader for PTY output
        let reader = pair.master.try_clone_reader().context("Failed to clone PTY reader")?;

        // Flag to track completion
        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();

        // Spawn task to read PTY output
        let event_tx_clone = event_tx.clone();
        let reader_handle = std::thread::spawn(move || {
            let buf_reader = BufReader::new(reader);
            let mut last_meaningful_line = String::new();

            for line in buf_reader.lines() {
                match line {
                    Ok(text) => {
                        // Strip ANSI escape codes for cleaner log display
                        let clean_text = strip_ansi_codes(&text);
                        let trimmed = clean_text.trim();

                        // Skip empty lines and repetitive UI elements
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Skip lines that are just box-drawing or UI decorations
                        if is_ui_decoration(trimmed) {
                            continue;
                        }

                        // Skip if same as last line (Claude TUI refreshes)
                        if trimmed == last_meaningful_line {
                            continue;
                        }

                        last_meaningful_line = trimmed.to_string();

                        // Determine event type based on content
                        let event = if trimmed.starts_with("⏺") || trimmed.starts_with("Done") || trimmed.starts_with("✓") {
                            LogEvent::system(truncate(trimmed, 150))
                        } else if trimmed.contains("Reading") || trimmed.contains("Writing") || trimmed.contains("Edit") {
                            LogEvent::tool_call("agent", truncate(trimmed, 150))
                        } else {
                            LogEvent::text(truncate(trimmed, 150))
                        };

                        let _ = event_tx_clone.blocking_send(event.for_job(job_id));
                    }
                    Err(_) => break,
                }
            }
            completed_clone.store(true, Ordering::SeqCst);
        });

        // Wait for child to exit
        let exit_status = child.wait().context("Failed to wait for agent")?;

        // Wait for reader to finish
        let _ = reader_handle.join();

        let mut result = AgentResult {
            success: exit_status.success(),
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
        };

        if exit_status.success() {
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
                .await;
        } else {
            result.error = Some(format!("Agent exited with: {:?}", exit_status));
            let _ = event_tx
                .send(
                    LogEvent::error(format!("Job #{} failed: {:?}", job_id, exit_status))
                        .for_job(job_id),
                )
                .await;
        }

        Ok(result)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        let binary = match self.cli_type {
            CliType::Claude => "claude",
            CliType::Codex => "codex",
            CliType::Gemini => "gemini",
            CliType::Custom => return false,
        };

        std::process::Command::new("which")
            .arg(binary)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (end of escape sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c.is_ascii_control() && c != '\n' && c != '\t' {
            // Skip other control characters
        } else {
            result.push(c);
        }
    }

    result
}

/// Truncate a string to a maximum length (char-safe)
fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// Check if a line is just UI decoration (box drawing, separators, etc.)
fn is_ui_decoration(s: &str) -> bool {
    // Line is mostly box-drawing characters or dashes
    let meaningful_chars = s.chars().filter(|c| {
        !matches!(c,
            '─' | '│' | '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' |
            '╭' | '╮' | '╰' | '╯' | '═' | '║' | '╔' | '╗' | '╚' | '╝' |
            '-' | '|' | '+' | '=' | ' ' | '▘' | '▝' | '▖' | '▗' |
            '⏵' // Permission indicator
        )
    }).count();

    // If less than 20% meaningful characters, it's decoration
    let total = s.chars().count();
    if total == 0 {
        return true;
    }

    meaningful_chars * 5 < total // Less than 20% meaningful
}
