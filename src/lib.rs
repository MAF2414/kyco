//! KYCo - Know Your Codebase
//!
//! KYCo executes code tasks via SDK-based agents through a local Bridge server.
//! Tasks are created through IDE extensions (VSCode, JetBrains) that send
//! selections and context to the GUI for processing.

pub mod agent;
pub mod cli;
pub mod config;
pub mod domain;
pub mod git;
pub mod gui;
pub mod job;
pub mod workspace;

pub use domain::*;
pub use workspace::{Workspace, WorkspaceId, WorkspaceRegistry};
