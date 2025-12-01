//! Agent execution and management

mod runner;
mod claude;
mod codex;
mod gemini;
mod terminal;
mod output;
mod registry;
mod chain;

pub use runner::{AgentRunner, AgentResult};
pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use gemini::GeminiAdapter;
pub use terminal::{get_session as get_terminal_session, TerminalAdapter, TerminalSession};
pub use output::StreamEvent;
pub use registry::AgentRegistry;
pub use chain::{ChainRunner, ChainResult, ChainStepResult};
