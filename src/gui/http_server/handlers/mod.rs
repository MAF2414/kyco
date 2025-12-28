//! HTTP request handlers for IDE and control API endpoints.

mod ide;
mod job_continue;
mod job_create;
mod job_delete;
mod job_lifecycle;
mod misc;

use crate::JobId;

// Re-export executor event for submodules
pub(super) use super::super::executor::ExecutorEvent;

// Re-export all handlers for use from parent module
pub use ide::{handle_batch_request, handle_selection_request};
pub use job_continue::handle_control_job_continue;
pub use job_create::handle_control_job_create;
pub use job_delete::handle_control_job_delete;
pub use job_lifecycle::{
    handle_control_job_abort, handle_control_job_get, handle_control_job_queue,
    handle_control_jobs_list,
};
pub use misc::{handle_control_config_reload, handle_control_log};

/// Parse job ID from URL path like `/ctl/jobs/123` or `/ctl/jobs/123/abort`.
pub(crate) fn parse_job_id_from_path(path: &str, suffix: Option<&str>) -> Result<JobId, &'static str> {
    let trimmed = path.trim_end_matches('/');
    let trimmed = match suffix {
        Some(suffix) => trimmed
            .strip_suffix(&format!("/{suffix}"))
            .ok_or("bad_path")?,
        None => trimmed,
    };

    let id_str = trimmed.rsplit('/').next().ok_or("bad_path")?;
    id_str.parse::<JobId>().map_err(|_| "bad_job_id")
}
