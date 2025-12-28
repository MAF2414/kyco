//! Terminal session management for REPL mode.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::helpers::{is_process_running, shell_escape};

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
        let script_file_str = script_file.display().to_string();
        // Script cleans up both the PID file and itself when done
        let script_content = format!(
            "#!/bin/bash\ncd '{}'\necho $$ > '{}'\n{}\nrm -f '{}'\nrm -f '{}'\n",
            cwd_str.replace('\'', "'\\''"),
            pid_file_str.replace('\'', "'\\''"),
            full_command,
            pid_file_str.replace('\'', "'\\''"),
            script_file_str.replace('\'', "'\\''"),
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
    pub fn is_running(&self) -> bool {
        if !self.running.load(Ordering::SeqCst) {
            return false;
        }

        let pid_file = std::env::temp_dir().join(format!("kyco_job_{}.pid", self.job_id));
        if !pid_file.exists() {
            self.running.store(false, Ordering::SeqCst);
            return false;
        }

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
    pub fn focus(&self) -> Result<()> {
        let status = Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .status()
            .context("Failed to focus terminal")?;

        if !status.success() {
            anyhow::bail!(
                "Failed to focus terminal: open command exited with {}",
                status
            );
        }

        Ok(())
    }

    /// Wait synchronously for the session to complete.
    ///
    /// Blocks the current thread, polling every 500ms until the agent
    /// process exits. Prefer using async polling in production code.
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

/// Global registry of active terminal sessions.
///
/// Maps job IDs to their corresponding [`TerminalSession`] handles.
/// Used by the TUI to look up sessions for focus commands.
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
/// The session handle if found, `None` otherwise.
pub fn get_session(job_id: u64) -> Option<Arc<TerminalSession>> {
    let sessions = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    sessions.get(&job_id).cloned()
}

/// Register a terminal session in the global registry.
///
/// Called automatically when a new session is spawned via [`TerminalAdapter::run`].
pub fn register_session(session: Arc<TerminalSession>) {
    let mut sessions = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    sessions.insert(session.job_id, session);
}

/// Remove a terminal session from the global registry.
///
/// Called automatically when a job completes.
pub fn unregister_session(job_id: u64) {
    let mut sessions = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    sessions.remove(&job_id);
}
