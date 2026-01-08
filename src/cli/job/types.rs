//! Type definitions for job control API responses.

use crate::{Job, JobId};

#[derive(Debug, serde::Deserialize)]
pub(super) struct JobsListResponse {
    pub jobs: Vec<Job>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct JobGetResponse {
    pub job: Job,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(super) struct JobCreateResponse {
    pub job_ids: Vec<JobId>,
    #[allow(dead_code)]
    pub group_id: Option<u64>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(super) struct JobContinueResponse {
    pub job_id: JobId,
}

/// Arguments for starting a new job via the CLI.
#[derive(Debug, Clone)]
pub struct JobStartArgs {
    pub file_path: Option<String>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub selected_text: Option<String>,
    pub mode: String,
    pub prompt: Option<String>,
    pub agent: Option<String>,
    pub agents: Vec<String>,
    pub queue: bool,
    pub force_worktree: bool,
    pub json: bool,
}
