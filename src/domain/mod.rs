//! Core domain types for KYCo

mod job;
mod comment;
mod agent;
mod scope;
mod target;
mod log_event;
mod agent_group;

pub use job::{Job, JobId, JobResult, JobStats, JobStatus};
pub use comment::{CommentTag, StatusMarker};
pub use agent::{
    AgentConfig, AgentMode, ClaudeAgentDefinition, CliType, McpServerConfig, ModeTemplate, SdkType,
    SessionMode, SystemPromptMode,
};
pub use scope::ScopeDefinition;
pub use target::Target;
pub use log_event::{LogEvent, LogEventKind};
pub use agent_group::{AgentGroupId, AgentRunGroup, GroupStatus};
