//! Executor event types for GUI communication

use crate::{ChainStepSummary, LogEvent};

/// Message to send back to GUI
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    /// Job started running
    JobStarted(u64),
    /// Job completed successfully
    JobCompleted(u64),
    /// Job failed with error
    JobFailed(u64, String),
    /// Chain step completed
    ChainStepCompleted {
        job_id: u64,
        step_index: usize,
        total_steps: usize,
        mode: String,
        state: Option<String>,
        /// Summary of the completed step for UI display
        step_summary: ChainStepSummary,
    },
    /// Chain completed
    ChainCompleted {
        job_id: u64,
        chain_name: String,
        steps_executed: usize,
        success: bool,
    },
    /// Log message
    Log(LogEvent),
    /// Permission request from Bridge (tool approval needed)
    PermissionNeeded {
        job_id: u64,
        request_id: String,
        session_id: String,
        tool_name: String,
        tool_input: std::collections::HashMap<String, serde_json::Value>,
    },
}
