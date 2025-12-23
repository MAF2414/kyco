//! Job management functionality for the GUI
//!
//! This module contains all job-related logic including:
//! - Job creation and management
//! - Job list rendering
//! - Job file I/O operations
//! - Multi-agent job creation

mod io;
mod list;
mod operations;

// Re-export all public items for backwards compatibility
pub use io::write_job_request;
pub use list::render_job_list;
pub use operations::{
    CreateJobsResult, apply_job, create_job_from_selection, create_jobs_from_selection_multi,
    kill_job, mark_job_complete, queue_job, refresh_jobs, reject_job,
};
