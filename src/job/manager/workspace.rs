//! Workspace-aware methods for JobManager (Phase 2: Multi-Workspace Support)

use std::path::PathBuf;

use anyhow::Result;

use crate::workspace::WorkspaceId;
use crate::{CommentTag, Job, JobId, JobStatus, ScopeDefinition};

use super::JobManager;

impl JobManager {
    /// Create a new job with workspace association
    pub fn create_job_for_workspace(
        &mut self,
        tag: &CommentTag,
        agent_id: &str,
        workspace_id: WorkspaceId,
        workspace_path: PathBuf,
    ) -> Result<JobId> {
        self.create_job_for_workspace_with_range(tag, agent_id, workspace_id, workspace_path, None)
    }

    /// Create a new job with workspace association and optional line range
    pub fn create_job_for_workspace_with_range(
        &mut self,
        tag: &CommentTag,
        agent_id: &str,
        workspace_id: WorkspaceId,
        workspace_path: PathBuf,
        line_end: Option<usize>,
    ) -> Result<JobId> {
        let id = self.allocate_id();

        // Always use file scope - the agent will determine the actual scope from context
        let scope_def = ScopeDefinition::file(tag.file_path.clone());

        let target = if let Some(end) = line_end {
            if end != tag.line_number {
                format!("{}:{}-{}", tag.file_path.display(), tag.line_number, end)
            } else {
                format!("{}:{}", tag.file_path.display(), tag.line_number)
            }
        } else {
            format!("{}:{}", tag.file_path.display(), tag.line_number)
        };

        let job = Job::new_with_workspace(
            id,
            workspace_id,
            workspace_path,
            tag.mode.clone(),
            scope_def,
            target,
            tag.description.clone(),
            agent_id.to_string(),
            tag.file_path.clone(),
            tag.line_number,
            if tag.raw_line.trim().is_empty() {
                None
            } else {
                Some(tag.raw_line.clone())
            },
        );

        self.insert_job(id, job);

        Ok(id)
    }

    pub fn jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id))
            .collect()
    }

    pub fn pending_jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id) && j.status == JobStatus::Pending)
            .collect()
    }

    pub fn running_jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id) && j.status == JobStatus::Running)
            .collect()
    }

    /// Returns legacy jobs (those without workspace association)
    pub fn jobs_without_workspace(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id.is_none())
            .collect()
    }

    pub fn set_job_workspace(
        &mut self,
        job_id: JobId,
        workspace_id: WorkspaceId,
        workspace_path: PathBuf,
    ) -> bool {
        if let Some(job) = self.jobs.get_mut(&job_id) {
            job.workspace_id = Some(workspace_id);
            job.workspace_path = Some(workspace_path);
            self.generation += 1;
            true
        } else {
            false
        }
    }
}
