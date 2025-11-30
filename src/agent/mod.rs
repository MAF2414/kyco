//! Agent execution and management

mod runner;
mod claude;
mod codex;
mod gemini;
mod pty;
mod pty_session;
mod terminal;
mod output;
mod registry;

pub use runner::{AgentRunner, AgentResult};
pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use gemini::GeminiAdapter;
pub use pty::PtyAdapter;
pub use pty_session::PtySession;
pub use terminal::{get_session as get_terminal_session, TerminalAdapter, TerminalSession};
pub use output::StreamEvent;
pub use registry::AgentRegistry;
