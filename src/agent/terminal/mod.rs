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
//! Active sessions are tracked in a global registry allowing
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

mod adapter;
mod helpers;
mod session;

pub use adapter::TerminalAdapter;
pub use session::{get_session, TerminalSession};
