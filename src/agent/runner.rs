//! Generic agent runner trait for executing coding agents.
//!
//! This module defines the [`AgentRunner`] trait that all agent adapters must implement,
//! enabling a unified interface for executing different AI coding agents (Claude, Codex, etc.).
//!
//! # Architecture
//!
//! The agent system follows an adapter pattern:
//! - [`AgentRunner`] defines the common interface for all agents
//! - [`AgentResult`] captures execution outcomes including success/failure, costs, and output
//! - Concrete adapters (e.g., `ClaudeAdapter`, `CodexAdapter`) implement `AgentRunner`
//!
//! # Example
//!
//! ```ignore
//! use coderail::agent::{AgentRunner, AgentResult};
//!
//! // Check if agent is available before running
//! if agent.is_available() {
//!     let result = agent.run(&job, &worktree_path, &config, event_tx).await?;
//!     if result.success {
//!         println!("Job completed! Cost: ${:.4}", result.cost_usd.unwrap_or(0.0));
//!     }
//! }
//! ```

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::sync::mpsc;

use crate::{AgentConfig, Job, LogEvent};

/// Result of an agent execution.
///
/// Contains all information about a completed agent run, including success status,
/// any errors, file changes, cost metrics, and the raw output text for parsing
/// structured result blocks (like `---kyco`).
#[derive(Debug)]
pub struct AgentResult {
    /// Whether the agent completed successfully.
    ///
    /// `true` indicates the agent finished without errors; `false` means the
    /// execution failed or was interrupted.
    pub success: bool,

    /// Error message if the execution failed.
    ///
    /// Contains diagnostic information when `success` is `false`.
    pub error: Option<String>,

    /// Files that were changed during execution.
    ///
    /// Populated by the agent based on tool calls (Write, Edit) or git diff analysis.
    pub changed_files: Vec<std::path::PathBuf>,

    /// Total cost in USD (if available).
    ///
    /// Reported by agents that track API usage costs (e.g., Claude).
    pub cost_usd: Option<f64>,

    /// Input tokens used.
    pub input_tokens: Option<u64>,

    /// Output tokens generated.
    pub output_tokens: Option<u64>,

    /// Cache read tokens (prompt caching).
    pub cache_read_tokens: Option<u64>,

    /// Cache write tokens (prompt caching).
    pub cache_write_tokens: Option<u64>,

    /// Duration in milliseconds.
    ///
    /// Wall-clock time from agent start to completion.
    pub duration_ms: Option<u64>,

    /// The prompt that was sent to the model.
    ///
    /// Useful for debugging and logging the exact instructions given to the agent.
    pub sent_prompt: Option<String>,

    /// The text output from the agent.
    ///
    /// Used for parsing structured output blocks like `---` that contain
    /// job result metadata (title, status, summary, state).
    pub output_text: Option<String>,

    /// SDK Structured Output (validated JSON from json_schema outputFormat).
    ///
    /// When the agent is configured with a JSON Schema via `structured_output_schema`,
    /// the SDK returns validated JSON in this field. Used for BugBounty findings/memory.
    pub structured_output: Option<serde_json::Value>,

    /// Session ID from the Bridge for session continuation.
    ///
    /// For session-mode jobs, this ID can be used to send follow-up prompts
    /// to continue the conversation.
    pub session_id: Option<String>,
}

/// Trait for agent adapters.
///
/// Implement this trait to add support for a new AI coding agent. Each adapter
/// is responsible for:
/// - Building the appropriate prompt from the job configuration
/// - Spawning and managing the agent process
/// - Streaming events back to the UI via the event channel
/// - Parsing the agent's output into an [`AgentResult`]
///
/// # Implementors
///
/// - `ClaudeAdapter` - Anthropic's Claude Code CLI
/// - `CodexAdapter` - OpenAI's Codex CLI
/// - `TerminalAdapter` - Interactive terminal/REPL mode
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Run a job using this agent.
    ///
    /// Executes the given job in the specified worktree directory, streaming
    /// progress events to the UI and returning the final result.
    ///
    /// # Arguments
    ///
    /// * `job` - The job to execute, containing mode, target, scope, and description
    /// * `worktree` - The path to the Git worktree where the agent should operate
    /// * `config` - Agent-specific configuration (binary path, args, env vars)
    /// * `event_tx` - Channel to send [`LogEvent`]s for real-time UI updates
    ///
    /// # Returns
    ///
    /// An [`AgentResult`] containing the execution outcome, or an error if the
    /// agent process could not be spawned.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The agent binary cannot be found or executed
    /// - The worktree path is invalid
    /// - Process spawning fails for other system reasons
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult>;

    /// Get the unique identifier for this agent.
    ///
    /// Returns a string like `"claude"` or `"codex"` that matches
    /// the agent ID used in configuration files.
    fn id(&self) -> &str;

    /// Check if this agent is available on the system.
    ///
    /// Typically verifies that the required binary exists and is executable.
    /// Used to filter available agents in the UI and prevent running jobs
    /// with unavailable agents.
    fn is_available(&self) -> bool;
}
