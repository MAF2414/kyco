//! KYCo - Know Your Codebase
//!
//! KYCo scans your codebase for special marker comments, converts them into jobs,
//! and executes them via external coding CLIs (like Claude Code). The AI explains
//! what it does, so you always understand your code - the antidote to vibe coding.
//!
//! ## Input Methods
//!
//! KYCo supports two ways to create code tasks:
//!
//! 1. **GUI (Primary)**: Global hotkey triggers a popup that reads the current
//!    selection via Accessibility APIs. Works with any editor/IDE.
//!
//! 2. **Comments (Fallback)**: Marker comments in code (e.g., `@@refactor`)
//!    are scanned and converted to jobs.

pub mod agent;
pub mod comment;
pub mod config;
pub mod domain;
pub mod git;
pub mod gui;
pub mod job;
pub mod scanner;
pub mod watcher;

pub use domain::*;
