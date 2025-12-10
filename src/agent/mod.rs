//! Agent execution and management.
//!
//! This module provides the core abstraction layer for executing AI coding agents.
//! It supports multiple agent backends (Claude, Codex, Gemini) through a unified
//! [`AgentRunner`] trait, enabling the system to dispatch jobs to different AI models
//! seamlessly.
//!
//! # Architecture
//!
//! The agent system is organized around these key components:
//!
//! - **[`AgentRunner`]** - The core trait that all agent adapters implement, defining
//!   how to execute a job and stream results.
//! - **Adapters** - Backend-specific implementations:
//!   - [`ClaudeAdapter`] - Anthropic's Claude models via Claude Code CLI
//!   - [`CodexAdapter`] - OpenAI's Codex/GPT models via OpenAI Codex CLI
//!   - [`GeminiAdapter`] - Google's Gemini models via Gemini CLI
//!   - [`TerminalAdapter`] - Direct terminal execution for custom commands
//! - **[`AgentRegistry`]** - Manages available agents and their configurations,
//!   providing lookup and instantiation.
//! - **[`ChainRunner`]** - Executes sequential chains of modes, passing context
//!   between steps for multi-stage workflows.
//!
//! # Example
//!
//! ```rust,ignore
//! use kyco::agent::{AgentRegistry, AgentResult};
//!
//! // Create a registry and get an agent adapter
//! let registry = AgentRegistry::new(&config);
//! let adapter = registry.get_adapter("claude")?;
//!
//! // Execute a job
//! let result: AgentResult = adapter.run(&job, &worktree, &agent_config, event_tx).await?;
//! ```

mod runner;
mod claude;
mod codex;
mod gemini;
mod terminal;
mod registry;
mod chain;

pub use runner::{AgentRunner, AgentResult};
pub use claude::{ClaudeAdapter, StreamEvent};
pub use codex::CodexAdapter;
pub use gemini::GeminiAdapter;
pub use terminal::{get_session as get_terminal_session, TerminalAdapter, TerminalSession};
pub use registry::AgentRegistry;
pub use chain::{ChainRunner, ChainResult, ChainStepResult};
