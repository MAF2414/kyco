//! PTY Session management for interactive agent sessions
//!
//! Manages running PTY sessions with terminal buffer and input support.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;


/// Terminal screen buffer - stores the last N lines of output
#[derive(Debug, Clone)]
pub struct TerminalBuffer {
    /// Lines of terminal output
    lines: VecDeque<String>,
    /// Maximum number of lines to keep
    max_lines: usize,
}

impl TerminalBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
        }
    }

    /// Add a line to the buffer
    pub fn push(&mut self, line: String) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Get all lines as a slice
    pub fn lines(&self) -> Vec<&str> {
        self.lines.iter().map(|s| s.as_str()).collect()
    }

    /// Get the last N lines
    pub fn last_n(&self, n: usize) -> Vec<&str> {
        self.lines.iter().rev().take(n).rev().map(|s| s.as_str()).collect()
    }

    /// Clear the buffer
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

/// A running PTY session
pub struct PtySession {
    /// Job ID this session belongs to
    pub job_id: u64,
    /// Terminal output buffer (shared with reader thread)
    buffer: Arc<Mutex<TerminalBuffer>>,
    /// PTY master for writing input
    master: Box<dyn MasterPty + Send>,
    /// Child process
    child: Box<dyn Child + Send + Sync>,
    /// Whether the session is still running
    running: Arc<Mutex<bool>>,
}

impl PtySession {
    /// Spawn a new PTY session for an agent
    pub fn spawn(
        job_id: u64,
        binary: &str,
        args: &[String],
        prompt: &str,
        cwd: &Path,
        env: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(binary);
        cmd.args(args);
        cmd.arg("--");
        cmd.arg(prompt);
        cmd.cwd(cwd);
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)?;
        let reader = pair.master.try_clone_reader()?;

        let buffer = Arc::new(Mutex::new(TerminalBuffer::new(100)));
        let running = Arc::new(Mutex::new(true));

        // Spawn reader thread
        let buffer_clone = buffer.clone();
        let running_clone = running.clone();
        thread::spawn(move || {
            let buf_reader = BufReader::new(reader);
            for line in buf_reader.lines() {
                match line {
                    Ok(text) => {
                        let clean = strip_ansi_codes(&text);
                        if !clean.trim().is_empty() {
                            if let Ok(mut buf) = buffer_clone.lock() {
                                buf.push(clean);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut r) = running_clone.lock() {
                *r = false;
            }
        });

        Ok(Self {
            job_id,
            buffer,
            master: pair.master,
            child,
            running,
        })
    }

    /// Check if the session is still running
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Get the terminal buffer
    pub fn get_buffer(&self) -> Vec<String> {
        self.buffer
            .lock()
            .map(|b| b.lines().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Get the last N lines from the terminal
    pub fn get_last_lines(&self, n: usize) -> Vec<String> {
        self.buffer
            .lock()
            .map(|b| b.last_n(n).iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Send input to the PTY
    pub fn send_input(&mut self, input: &str) -> anyhow::Result<()> {
        let mut writer = self.master.take_writer()?;
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    /// Send a line of input (adds newline)
    pub fn send_line(&mut self, input: &str) -> anyhow::Result<()> {
        self.send_input(&format!("{}\n", input))
    }

    /// Wait for the child to exit
    pub fn wait(&mut self) -> anyhow::Result<bool> {
        let status = self.child.wait()?;
        if let Ok(mut r) = self.running.lock() {
            *r = false;
        }
        Ok(status.success())
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
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c == '\r' {
            // Skip carriage return
        } else if c.is_ascii_control() && c != '\n' && c != '\t' {
            // Skip other control characters
        } else {
            result.push(c);
        }
    }

    result
}
