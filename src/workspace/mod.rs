//! Workspace management for multi-repository support
//!
//! The workspace module provides:
//! - WorkspaceId: Unique identifier for each workspace
//! - Workspace: Configuration and state for a single repository/project
//! - WorkspaceRegistry: Manages multiple workspaces in a single KYCO instance

mod registry;

pub use registry::{Workspace, WorkspaceId, WorkspaceRegistry};
