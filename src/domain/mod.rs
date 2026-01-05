//! Core domain types for KYCo

mod agent;
mod agent_group;
mod comment;
mod job;
mod log_event;
mod scope;
mod target;

pub use agent::{
    AgentConfig, ClaudeAgentDefinition, CliType, McpServerConfig, ModeTemplate, SdkType,
    SystemPromptMode,
};
pub use agent_group::{AgentGroupId, AgentRunGroup, GroupStatus};
pub use comment::{CommentTag, StatusMarker};
pub use job::{ChainStepSummary, Job, JobId, JobResult, JobStats, JobStatus};
pub use log_event::{LogEvent, LogEventKind};
pub use scope::ScopeDefinition;
pub use target::Target;
