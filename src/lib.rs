//! KYCo - Know Your Codebase
//!
//! KYCo scans your codebase for special marker comments, converts them into jobs,
//! and executes them via external coding CLIs (like Claude Code). The AI explains
//! what it does, so you always understand your code - the antidote to vibe coding.

pub mod agent;
pub mod comment;
pub mod config;
pub mod domain;
pub mod git;
pub mod job;
pub mod scanner;
pub mod tui;
pub mod watcher;

pub use domain::*;
