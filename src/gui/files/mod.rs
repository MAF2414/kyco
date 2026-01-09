//! File search and batch selection module
//!
//! Provides a GUI view for:
//! - Searching files by glob patterns or content (grep)
//! - Batch selecting multiple files
//! - Previewing files and selecting specific lines
//! - Creating batch jobs or setting files as context

mod search;
mod state;

pub use search::perform_search;
pub use state::{FileMatch, FileSearchState, FileSelection, SearchMode};
