//! Core domain types for KYCo

mod job;
mod comment;
mod agent;
mod scope;
mod target;
mod log_event;

pub use job::{Job, JobId, JobStatus};
pub use comment::{CommentTag, StatusMarker};
pub use agent::{AgentConfig, AgentMode, CliType, SystemPromptMode, ModeTemplate};
pub use scope::{Scope, ScopeDefinition};
pub use target::Target;
pub use log_event::{LogEvent, LogEventKind};
