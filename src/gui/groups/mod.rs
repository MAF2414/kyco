//! Group management UI for multi-agent parallel execution
//!
//! This module provides the UI components for:
//! - Comparison popup for selecting between agent results
//! - Group operations (merge, cleanup)
//! - Visual indicators for grouped jobs in the job list

mod comparison;
mod operations;

pub use comparison::{render_comparison_popup, ComparisonAction, ComparisonState};
pub use operations::{merge_and_cleanup, GroupOperationResult};
