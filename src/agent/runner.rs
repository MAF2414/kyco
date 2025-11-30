//! Generic agent runner trait

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::sync::mpsc;

use crate::{AgentConfig, Job, LogEvent};

/// Result of an agent execution
#[derive(Debug)]
pub struct AgentResult {
    /// Whether the agent completed successfully
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Files that were changed
    pub changed_files: Vec<std::path::PathBuf>,

    /// Total cost in USD (if available)
    pub cost_usd: Option<f64>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// The prompt that was sent to the model
    pub sent_prompt: Option<String>,
}

/// Trait for agent adapters
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Run a job using this agent
    ///
    /// # Arguments
    /// * `job` - The job to execute
    /// * `worktree` - The path to the Git worktree for this job
    /// * `config` - The agent configuration
    /// * `event_tx` - Channel to send log events for UI updates
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult>;

    /// Get the agent ID
    fn id(&self) -> &str;

    /// Check if this agent is available (binary exists)
    fn is_available(&self) -> bool;
}
