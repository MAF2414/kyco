//! SDK Bridge integration.
//!
//! This module provides agent adapters that use the SDK Bridge for full
//! programmatic control over Claude Agent SDK and Codex SDK.
//!
//! # Architecture
//!
//! The bridge is a Node.js sidecar service that wraps the TypeScript SDKs:
//! - `@anthropic-ai/claude-agent-sdk` for Claude
//! - `@openai/codex-sdk` for Codex
//!
//! Communication happens via HTTP with NDJSON streaming for events.
//!
//! # Features
//!
//! The bridge provides capabilities not available via CLI:
//! - **Hooks**: PreToolUse/PostToolUse for validation and logging
//! - **Session Resume**: Continue conversations across KYCO restarts
//! - **Structured Output**: JSON Schema validation for responses
//! - **Custom Permissions**: Fine-grained tool access control
//! - **Token Tracking**: Accurate usage and cost metrics
//!
//! # Usage
//!
//! ```rust,ignore
//! use kyco::agent::bridge::{BridgeProcess, ClaudeBridgeAdapter};
//!
//! // Start the bridge server (once at app startup)
//! let bridge = BridgeProcess::spawn()?;
//!
//! // Use the bridge adapter like any other agent
//! let adapter = ClaudeBridgeAdapter::new();
//! let result = adapter.run(&job, &worktree, &config, event_tx).await?;
//! ```

mod adapter;
mod client;
mod types;

pub use adapter::{ClaudeBridgeAdapter, CodexBridgeAdapter};
pub use client::{BridgeClient, BridgeProcess};
pub use types::*;
