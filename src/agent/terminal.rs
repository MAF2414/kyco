//! Default terminal integration for REPL mode on macOS
//!
//! Spawns agent processes in a new terminal window (foreground),
//! tracks them via PID, and detects completion when the process exits.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

use super::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, CliType, Job, LogEvent};

/// A running terminal session
#[derive(Debug)]
pub struct TerminalSession {
    /// Job ID this session belongs to
    pub job_id: u64,
    /// Process ID of the shell running the agent
    pub pid: Option<u32>,
    /// Whether the session is still running
    running: Arc<AtomicBool>,
}

impl TerminalSession {
    /// Spawn a new terminal session for an agent
    ///
    /// Opens the default terminal in the foreground, runs the command,
    /// and returns a session handle for tracking.
    pub fn spawn(
        job_id: u64,
        binary: &str,
        args: &[String],
        prompt: &str,
        cwd: &Path,
    ) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));

        // Build the full command string with shell escaping
        let mut cmd_parts = vec![shell_escape(binary)];
        cmd_parts.extend(args.iter().map(|arg| shell_escape(arg)));
        cmd_parts.push(shell_escape("--"));
        cmd_parts.push(shell_escape(prompt));
        let full_command = cmd_parts.join(" ");

        // PID file for tracking when process exits
        let pid_file = std::env::temp_dir().join(format!("kyco_job_{}.pid", job_id));
        let pid_file_str = pid_file.display().to_string();

        // Ensure we have an absolute path
        let cwd_abs = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| cwd.to_path_buf())
                .join(cwd)
        };
        let cwd_str = cwd_abs.display().to_string();

        // Create a temporary shell script that does everything
        let script_file = std::env::temp_dir().join(format!("kyco_job_{}.sh", job_id));
        let script_content = format!(
            "#!/bin/bash\ncd '{}'\necho $$ > '{}'\n{}\nrm -f '{}'\n",
            cwd_str.replace('\'', "'\\''"),
            pid_file_str.replace('\'', "'\\''"),
            full_command,
            pid_file_str.replace('\'', "'\\''"),
        );

        std::fs::write(&script_file, &script_content)
            .context("Failed to write shell script")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_file)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_file, perms)?;
        }

        let script_path = script_file.display().to_string();

        // Use AppleScript to run the script in Terminal
        // Simple command: just the path to our script
        let applescript = format!(
            "tell application \"Terminal\"\n\tactivate\n\tdo script \"{}\"\nend tell",
            script_path.replace('\\', "\\\\").replace('"', "\\\"")
        );

        let status = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .status()
            .context("Failed to run AppleScript")?;

        if !status.success() {
            anyhow::bail!("AppleScript failed with status: {}", status);
        }

        // Wait a moment for PID file to be written
        std::thread::sleep(Duration::from_millis(1500));

        // Read the PID from the temp file
        let pid = std::fs::read_to_string(&pid_file)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok());

        Ok(Self {
            job_id,
            pid,
            running,
        })
    }

    /// Check if the session is still running
    pub fn is_running(&self) -> bool {
        if !self.running.load(Ordering::SeqCst) {
            return false;
        }

        // Check if the PID file still exists (deleted on exit)
        let pid_file = std::env::temp_dir().join(format!("kyco_job_{}.pid", self.job_id));
        if !pid_file.exists() {
            self.running.store(false, Ordering::SeqCst);
            return false;
        }

        // Also check if the process is still alive
        if let Some(pid) = self.pid {
            if !is_process_running(pid) {
                self.running.store(false, Ordering::SeqCst);
                return false;
            }
        }

        true
    }

    /// Focus the terminal window (bring to front)
    pub fn focus(&self) -> Result<()> {
        // Just activate Terminal app - it will bring the most recent window to front
        Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .spawn()
            .context("Failed to focus terminal")?;

        Ok(())
    }

    /// Wait for the session to complete
    pub fn wait(&self) -> bool {
        while self.is_running() {
            std::thread::sleep(Duration::from_millis(500));
        }
        true // Assume success if we got here
    }

    /// Mark the session as completed
    pub fn mark_completed(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Check if a process with the given PID is still running
fn is_process_running(pid: u32) -> bool {
    // Use kill -0 to check if process exists
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Escape a string for shell use
fn shell_escape(s: &str) -> String {
    // Wrap in single quotes and escape any single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Quote a string for AppleScript
/// Uses the 'quoted form of' equivalent: wrap in double quotes and escape
#[allow(dead_code)]
fn applescript_quote(s: &str) -> String {
    // Escape backslashes first, then double quotes
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Global registry of active terminal sessions
/// Used to look up sessions for focus commands
static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<u64, Arc<TerminalSession>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Get a terminal session by job ID
pub fn get_session(job_id: u64) -> Option<Arc<TerminalSession>> {
    SESSIONS.lock().ok()?.get(&job_id).cloned()
}

/// Register a terminal session
fn register_session(session: Arc<TerminalSession>) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.insert(session.job_id, session);
    }
}

/// Unregister a terminal session
fn unregister_session(job_id: u64) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.remove(&job_id);
    }
}

/// Terminal-based agent adapter for REPL mode
///
/// Spawns agents in a separate Terminal.app window (minimized by default).
/// The TUI can focus the terminal window when the user selects the job.
pub struct TerminalAdapter {
    id: String,
    cli_type: CliType,
}

use crate::SystemPromptMode;

impl TerminalAdapter {
    /// Create a new terminal adapter for a given CLI type
    pub fn new(id: impl Into<String>, cli_type: CliType) -> Self {
        Self {
            id: id.into(),
            cli_type,
        }
    }

    /// Create a terminal adapter for Claude
    pub fn claude() -> Self {
        Self::new("claude-terminal", CliType::Claude)
    }

    /// Create a terminal adapter for Codex
    pub fn codex() -> Self {
        Self::new("codex-terminal", CliType::Codex)
    }

    /// Create a terminal adapter for Gemini
    pub fn gemini() -> Self {
        Self::new("gemini-terminal", CliType::Gemini)
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

    /// Build command arguments including system prompt for REPL mode
    fn build_repl_args(&self, job: &Job, config: &AgentConfig) -> Vec<String> {
        let mut args = config.get_repl_args();

        // Add system prompt if configured
        if let Some(system_prompt) = self.build_system_prompt(job, config) {
            match config.system_prompt_mode {
                SystemPromptMode::Append => {
                    // For Claude in REPL mode
                    if self.cli_type == CliType::Claude {
                        args.push("--append-system-prompt".to_string());
                        args.push(system_prompt);
                    }
                }
                SystemPromptMode::Replace => {
                    // For Gemini or when explicitly replacing
                    if self.cli_type == CliType::Claude {
                        args.push("--system-prompt".to_string());
                        args.push(system_prompt);
                    } else if self.cli_type == CliType::Gemini {
                        // Gemini uses different mechanism (GEMINI.md files)
                        // System prompt passed differently - for now skip
                    }
                }
                SystemPromptMode::ConfigOverride => {
                    // For Codex - system prompt is handled via config
                    // In REPL mode, we can try passing it as append for Claude-like CLIs
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
            args.push(config.disallowed_tools.join(","));
        }

        // Add allowed tools if any
        if !config.allowed_tools.is_empty() && self.cli_type == CliType::Claude {
            args.push("--allowedTools".to_string());
            args.push(config.allowed_tools.join(","));
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

        // Log start
        let _ = event_tx
            .send(LogEvent::system(format!("Starting terminal job #{}", job_id)).for_job(job_id))
            .await;

        // Build args with system prompt included
        let repl_args = self.build_repl_args(job, config);

        // Spawn in terminal
        let session = TerminalSession::spawn(
            job_id,
            &config.binary,
            &repl_args,
            &prompt,
            worktree,
        )?;

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

        // Wait for completion
        handle.await?;

        // Cleanup
        unregister_session(job_id);

        let _ = event_tx
            .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
            .await;

        Ok(AgentResult {
            success: true,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
            output_text: None,
        })
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        // Check if we're on macOS and Terminal.app is available
        #[cfg(target_os = "macos")]
        {
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

        #[cfg(not(target_os = "macos"))]
        {
            false // Terminal adapter only available on macOS for now
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
