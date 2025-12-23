//! Default terminal integration for REPL mode on macOS.
//!
//! This module provides the [`TerminalAdapter`] for spawning AI agent processes
//! (Claude, Codex) in a separate Terminal.app window. This is useful
//! for REPL-style interactions where the user can see and interact with the
//! agent's output in real-time.
//!
//! # Architecture
//!
//! The terminal integration works by:
//! 1. Creating a temporary shell script with the agent command
//! 2. Using AppleScript to open Terminal.app and execute the script
//! 3. Writing the shell's PID to a temp file for tracking
//! 4. Polling the PID file and process status to detect completion
//!
//! # Session Management
//!
//! Active sessions are tracked in a global registry ([`SESSIONS`]) allowing
//! the TUI to focus specific terminal windows by job ID.
//!
//! # Platform Support
//!
//! Currently only supports macOS with Terminal.app. The adapter reports
//! unavailable on other platforms.
//!
//! # Example
//!
//! ```ignore
//! use crate::agent::TerminalAdapter;
//!
//! // Create an adapter for Claude CLI
//! let adapter = TerminalAdapter::claude();
//!
//! // Check availability before use
//! if adapter.is_available() {
//!     // Run a job (normally called via AgentRunner trait)
//!     let result = adapter.run(&job, &worktree, &config, event_tx).await?;
//! }
//! ```

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

/// A running terminal session representing an agent executing in Terminal.app.
///
/// Each session tracks a single job's execution, including its process ID
/// for lifecycle monitoring. Sessions are registered globally and can be
/// retrieved by job ID for operations like focusing the terminal window.
///
/// # Fields
///
/// - `job_id`: Unique identifier linking this session to a [`Job`]
/// - `pid`: Process ID of the shell (if successfully captured)
/// - `running`: Atomic flag for thread-safe status checking
///
/// # Lifecycle
///
/// 1. Created via [`TerminalSession::spawn`]
/// 2. Registered in global [`SESSIONS`] map
/// 3. Polled via [`is_running`](Self::is_running) until completion
/// 4. Unregistered when the agent finishes
#[derive(Debug)]
pub struct TerminalSession {
    /// Job ID this session belongs to
    pub job_id: u64,
    /// Process ID of the shell running the agent (captured from temp PID file)
    pub pid: Option<u32>,
    /// Whether the session is still running (atomic for thread-safe access)
    running: Arc<AtomicBool>,
}

impl TerminalSession {
    /// Spawn a new terminal session for an agent.
    ///
    /// Opens Terminal.app in the foreground, executes the agent command,
    /// and returns a session handle for tracking the process lifecycle.
    ///
    /// # Arguments
    ///
    /// * `job_id` - Unique job identifier for tracking
    /// * `binary` - Path to the agent CLI binary (e.g., "claude", "codex")
    /// * `args` - Additional CLI arguments (system prompts, tool configs, etc.)
    /// * `prompt` - The task prompt to send to the agent
    /// * `cwd` - Working directory for the agent process
    ///
    /// # Returns
    ///
    /// A [`TerminalSession`] handle, or an error if spawning failed.
    ///
    /// # Implementation Details
    ///
    /// 1. Builds a shell script that writes its PID to a temp file
    /// 2. Uses AppleScript to open Terminal.app and run the script
    /// 3. Waits briefly (1.5s) for the PID file to be written
    /// 4. Returns the session with the captured PID
    ///
    /// # Errors
    ///
    /// - Script file creation fails
    /// - AppleScript execution fails
    /// - Terminal.app is not available
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

        std::fs::write(&script_file, &script_content).context("Failed to write shell script")?;

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

    /// Check if the session is still running.
    ///
    /// Uses a two-phase check:
    /// 1. Checks if the PID file still exists (deleted when script exits)
    /// 2. Verifies the process is alive via `kill -0`
    ///
    /// Updates the internal `running` flag atomically on state change.
    ///
    /// # Returns
    ///
    /// `true` if the agent process is still executing, `false` otherwise.
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

    /// Focus the terminal window (bring to front).
    ///
    /// Activates Terminal.app, bringing the most recent window to the foreground.
    /// Note: This brings Terminal.app to focus but doesn't specifically select
    /// this job's tab/window if multiple are open.
    ///
    /// # Errors
    ///
    /// Returns an error if the `open` command fails to execute.
    pub fn focus(&self) -> Result<()> {
        // Just activate Terminal app - it will bring the most recent window to front
        Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .spawn()
            .context("Failed to focus terminal")?;

        Ok(())
    }

    /// Wait synchronously for the session to complete.
    ///
    /// Blocks the current thread, polling every 500ms until the agent
    /// process exits. Prefer using async polling in production code.
    ///
    /// # Returns
    ///
    /// Always returns `true` (success assumed on completion).
    pub fn wait(&self) -> bool {
        while self.is_running() {
            std::thread::sleep(Duration::from_millis(500));
        }
        true // Assume success if we got here
    }

    /// Mark the session as completed manually.
    ///
    /// Sets the internal `running` flag to `false`. Useful for external
    /// cancellation or cleanup scenarios.
    pub fn mark_completed(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Check if a process with the given PID is still running.
///
/// Uses `kill -0` which sends no signal but checks process existence.
/// This is a POSIX-standard way to verify a process is alive.
///
/// # Arguments
///
/// * `pid` - The process ID to check
///
/// # Returns
///
/// `true` if the process exists and is accessible, `false` otherwise.
fn is_process_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Escape a string for safe shell use.
///
/// Wraps the string in single quotes and escapes embedded single quotes
/// using the `'\''` technique (end quote, escaped quote, start quote).
///
/// # Example
///
/// ```ignore
/// assert_eq!(shell_escape("hello"), "'hello'");
/// assert_eq!(shell_escape("it's"), "'it'\\''s'");
/// ```
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Quote a string for AppleScript embedding.
///
/// Escapes backslashes and double quotes, then wraps in double quotes.
/// Equivalent to AppleScript's `quoted form of` for string literals.
///
/// # Arguments
///
/// * `s` - The string to quote
///
/// # Returns
///
/// A properly escaped string safe for AppleScript interpolation.
#[allow(dead_code)]
fn applescript_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Global registry of active terminal sessions.
///
/// Maps job IDs to their corresponding [`TerminalSession`] handles.
/// Used by the TUI to look up sessions for focus commands (e.g., when
/// a user presses 'f' to bring a job's terminal to the foreground).
///
/// Thread-safe via [`Mutex`] wrapping.
static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<u64, Arc<TerminalSession>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Get a terminal session by job ID.
///
/// # Arguments
///
/// * `job_id` - The job ID to look up
///
/// # Returns
///
/// The session handle if found and the mutex is accessible, `None` otherwise.
pub fn get_session(job_id: u64) -> Option<Arc<TerminalSession>> {
    SESSIONS.lock().ok()?.get(&job_id).cloned()
}

/// Register a terminal session in the global registry.
///
/// Called automatically when a new session is spawned via [`TerminalAdapter::run`].
fn register_session(session: Arc<TerminalSession>) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.insert(session.job_id, session);
    }
}

/// Remove a terminal session from the global registry.
///
/// Called automatically when a job completes.
fn unregister_session(job_id: u64) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.remove(&job_id);
    }
}

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
/// # Usage
///
/// ```ignore
/// // Create adapter for specific CLI
/// let adapter = TerminalAdapter::claude();
///
/// // Or use generic constructor
/// let adapter = TerminalAdapter::new("my-claude", CliType::Claude);
///
/// // Check availability (verifies binary exists)
/// if adapter.is_available() {
///     // Use via AgentRunner trait
/// }
/// ```
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

use crate::SystemPromptMode;

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
        let template = config.get_mode_template(&job.mode);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");

        // Replace template placeholders
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

    /// Build command arguments including system prompt for REPL mode.
    ///
    /// Constructs the CLI arguments based on the agent type and configuration:
    /// - Adds system prompt flags (`--append-system-prompt` or `--system-prompt`)
    /// - Adds tool restrictions (`--disallowedTools`, `--allowedTools`)
    ///
    /// The handling varies by CLI type due to different CLI interfaces.
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
                    // For Claude or when explicitly replacing
                    if self.cli_type == CliType::Claude {
                        args.push("--system-prompt".to_string());
                        args.push(system_prompt);
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

        // Add disallowed tools if any (each tool as a separate argument)
        if !config.disallowed_tools.is_empty() && self.cli_type == CliType::Claude {
            args.push("--disallowedTools".to_string());
            for tool in &config.disallowed_tools {
                args.push(tool.clone());
            }
        }

        // Add allowed tools if any (each tool as a separate argument)
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

        // Log start
        let _ = event_tx
            .send(LogEvent::system(format!("Starting terminal job #{}", job_id)).for_job(job_id))
            .await;

        // Build args with system prompt included
        let repl_args = self.build_repl_args(job, config);

        // Spawn in terminal
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
            session_id: None, // Terminal mode doesn't support session continuation
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}
