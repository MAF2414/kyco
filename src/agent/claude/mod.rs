//! Claude Code agent module
//!
//! This module contains the Claude Code CLI adapter and related output parsing types.

mod adapter;
mod output;

pub use adapter::ClaudeAdapter;
pub use output::StreamEvent;
