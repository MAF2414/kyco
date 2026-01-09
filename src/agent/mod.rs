//! Agent execution and management.
//!
//! This module provides the core abstraction layer for executing AI coding agents.
//! Agents run primarily via local CLI adapters (Codex CLI / Claude Code).
//! An optional Bridge server is available for SDK-style integrations.
//!
//! # Architecture
//!
//! The agent system is organized around these key components:
//!
//! - **[`AgentRunner`]** - The core trait that all agent adapters implement, defining
//!   how to execute a job and stream results.
//! - **CLI Adapters** - Backend-specific implementations:
//!   - [`ClaudeAdapter`] - Claude Code CLI
//!   - [`CodexAdapter`] - Codex CLI
//! - **Bridge Adapters (optional)** - SDK-style session control:
//!   - [`ClaudeBridgeAdapter`]
//!   - [`CodexBridgeAdapter`]
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

pub mod bridge;
mod chain;
pub mod process_registry;
mod registry;
mod runner;

mod claude;
mod codex;
mod terminal;

pub use bridge::{BridgeClient, BridgeProcess, ClaudeBridgeAdapter, CodexBridgeAdapter};
pub use chain::{ChainProgressEvent, ChainResult, ChainRunner, ChainStepResult};
pub use registry::{AgentRegistry, DEFAULT_TERMINAL_SUFFIX};
pub use runner::{AgentResult, AgentRunner};

pub use claude::{ClaudeAdapter, StreamEvent};
pub use codex::CodexAdapter;
#[deprecated(note = "Legacy interactive terminal adapter; prefer CLI adapters")]
pub use terminal::{TerminalAdapter, TerminalSession, get_session as get_terminal_session};
