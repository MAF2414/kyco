//! Agent execution and management.
//!
//! This module provides the core abstraction layer for executing AI coding agents.
//! Agents run through the local Bridge server for SDK-based control.
//!
//! # Architecture
//!
//! The agent system is organized around these key components:
//!
//! - **[`AgentRunner`]** - The core trait that all agent adapters implement, defining
//!   how to execute a job and stream results.
//! - **SDK Adapters** - Backend-specific implementations:
//!   - [`ClaudeBridgeAdapter`] - Anthropic's Claude via Claude Agent SDK
//!   - [`CodexBridgeAdapter`] - OpenAI's Codex via Codex SDK
//! - **[`AgentRegistry`]** - Manages available agents and selects an adapter per job.
//! - **[`ChainRunner`]** - Executes sequential chains of modes, passing context
//!   between steps for multi-stage workflows.
//!
//! # Example
//!
//! ```rust,ignore
//! use kyco::agent::{AgentRegistry, AgentResult};
//!
//! // Create a registry and get an agent adapter
//! let registry = AgentRegistry::new();
//! let adapter = registry.get("claude")?;
//!
//! // Execute a job
//! let result: AgentResult = adapter.run(&job, &worktree, &agent_config, event_tx).await?;
//! ```

mod runner;
mod registry;
mod chain;
pub mod bridge;

// Legacy modules - kept for backwards compatibility but deprecated
mod claude;
mod codex;
mod terminal;

pub use runner::{AgentRunner, AgentResult};
pub use registry::{AgentRegistry, DEFAULT_TERMINAL_SUFFIX};
pub use chain::{ChainRunner, ChainResult, ChainStepResult, ChainProgressEvent};
pub use bridge::{BridgeClient, BridgeProcess, ClaudeBridgeAdapter, CodexBridgeAdapter};

// Legacy exports - deprecated, use bridge adapters instead
#[deprecated(note = "Use ClaudeBridgeAdapter instead")]
pub use claude::{ClaudeAdapter, StreamEvent};
#[deprecated(note = "Use CodexBridgeAdapter instead")]
pub use codex::CodexAdapter;
#[deprecated(note = "Sessions are now handled by SDK adapters")]
pub use terminal::{get_session as get_terminal_session, TerminalAdapter, TerminalSession};
