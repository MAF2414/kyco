//! Types for job list filtering and actions

use crate::{Job, JobId, JobStatus};

/// Filter options for job list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JobListFilter {
    /// Show all jobs
    #[default]
    All,
    /// Show only active jobs (Running, Blocked, Queued, Pending)
    Active,
    /// Show only finished jobs (Done, Failed, Rejected, Merged)
    Finished,
    /// Show only failed jobs
    Failed,
}

impl JobListFilter {
    /// Check if a job matches this filter
    pub fn matches(&self, job: &Job) -> bool {
        match self {
            JobListFilter::All => true,
            JobListFilter::Active => !job.is_finished(),
            JobListFilter::Finished => job.is_finished(),
            JobListFilter::Failed => job.status == JobStatus::Failed,
        }
    }

    /// Get display label for this filter
    pub fn label(&self) -> &'static str {
        match self {
            JobListFilter::All => "All",
            JobListFilter::Active => "Active",
            JobListFilter::Finished => "Done",
            JobListFilter::Failed => "Failed",
        }
    }

    /// Get count of jobs matching this filter
    pub fn count(&self, jobs: &[Job]) -> usize {
        jobs.iter().filter(|j| self.matches(j)).count()
    }
}

/// Action returned from job list rendering
#[derive(Debug, Clone)]
pub enum JobListAction {
    /// No action
    None,
    /// Delete the specified job
    DeleteJob(JobId),
    /// Delete all finished jobs
    DeleteAllFinished,
}
