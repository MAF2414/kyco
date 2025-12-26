//! Job manager implementation

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::git::find_git_root;
use crate::workspace::WorkspaceId;
use crate::{CommentTag, Job, JobId, JobStatus, ScopeDefinition};

/// Manages job lifecycle (in-memory only, no persistence)
pub struct JobManager {
    /// Root directory of the repository
    #[allow(dead_code)]
    root: PathBuf,

    /// All known jobs
    jobs: HashMap<JobId, Job>,

    /// Next job ID
    next_id: AtomicU64,

    /// File locks for concurrent job isolation
    file_locks: HashMap<PathBuf, JobId>,

    /// Generation counter - incremented on any mutation.
    /// Used by GUI to know when to refresh cached jobs.
    generation: u64,
}

impl JobManager {
    /// Create a new job manager
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();

        Self {
            root,
            jobs: HashMap::new(),
            next_id: AtomicU64::new(1),
            file_locks: HashMap::new(),
            generation: 0,
        }
    }

    /// Get current generation counter.
    /// Use this to check if jobs have changed since last refresh.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Create a new job manager (for API compatibility, same as new())
    pub fn load(root: &Path) -> Result<Self> {
        Ok(Self::new(root))
    }

    /// Create a new job from a comment tag
    pub fn create_job(&mut self, tag: &CommentTag, agent_id: &str) -> Result<JobId> {
        self.create_job_with_range(tag, agent_id, None)
    }

    /// Create a new job from a comment tag with optional line range
    pub fn create_job_with_range(
        &mut self,
        tag: &CommentTag,
        agent_id: &str,
        line_end: Option<usize>,
    ) -> Result<JobId> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Always use file scope - the agent will determine the actual scope from context
        let scope_def = ScopeDefinition::file(tag.file_path.clone());

        // Format target with line range if available
        let target = if let Some(end) = line_end {
            if end != tag.line_number {
                format!("{}:{}-{}", tag.file_path.display(), tag.line_number, end)
            } else {
                format!("{}:{}", tag.file_path.display(), tag.line_number)
            }
        } else {
            format!("{}:{}", tag.file_path.display(), tag.line_number)
        };

        // Determine workspace_path: git root > file's parent directory
        let workspace_path = find_git_root(&tag.file_path)
            .or_else(|| tag.file_path.parent().map(PathBuf::from));

        let mut job = Job::new(
            id,
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

        // Set workspace_path for proper job isolation
        job.workspace_path = workspace_path;

        self.jobs.insert(id, job);
        self.generation += 1;

        Ok(id)
    }

    /// Get a job by ID
    pub fn get(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    /// Get a mutable job by ID
    pub fn get_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    /// Get all jobs
    pub fn jobs(&self) -> Vec<&Job> {
        self.jobs.values().collect()
    }

    /// Get all pending jobs
    pub fn pending_jobs(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .collect()
    }

    /// Get all running jobs
    pub fn running_jobs(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::Running)
            .collect()
    }

    /// Update job status
    pub fn set_status(&mut self, id: JobId, status: JobStatus) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.set_status(status);
            self.generation += 1;
        }
    }

    /// Manually increment the generation counter.
    ///
    /// Call this after directly modifying a job via `get_mut()` to ensure
    /// the GUI cache is invalidated and refreshes the job list.
    pub fn touch(&mut self) {
        self.generation += 1;
    }

    /// Try to acquire a file lock for a job
    pub fn try_lock_file(&mut self, path: &Path, job_id: JobId) -> bool {
        if self.file_locks.contains_key(path) {
            false
        } else {
            self.file_locks.insert(path.to_path_buf(), job_id);
            true
        }
    }

    /// Release a file lock
    pub fn release_lock(&mut self, path: &Path) {
        self.file_locks.remove(path);
    }

    /// Release all locks held by a job
    pub fn release_job_locks(&mut self, job_id: JobId) {
        self.file_locks.retain(|_, id| *id != job_id);
    }

    /// Check if a file is locked by another job
    pub fn is_file_locked(&self, path: &Path, exclude_job: Option<JobId>) -> bool {
        match self.file_locks.get(path) {
            Some(id) if exclude_job.map_or(true, |ej| *id != ej) => true,
            _ => false,
        }
    }

    /// Check if a file is locked and return the blocking job ID
    pub fn get_blocking_job(&self, path: &Path, exclude_job: Option<JobId>) -> Option<JobId> {
        match self.file_locks.get(path) {
            Some(&id) if exclude_job.map_or(true, |ej| id != ej) => Some(id),
            _ => None,
        }
    }

    /// Get all file locks held by a specific job
    pub fn get_job_locks(&self, job_id: JobId) -> Vec<PathBuf> {
        self.file_locks
            .iter()
            .filter(|&(_, &id)| id == job_id)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Remove a job from the manager.
    ///
    /// This also releases any file locks held by the job.
    pub fn remove_job(&mut self, job_id: JobId) -> Option<Job> {
        self.release_job_locks(job_id);
        let removed = self.jobs.remove(&job_id);
        if removed.is_some() {
            self.generation += 1;
        }
        removed
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Workspace-aware methods (Phase 2: Multi-Workspace Support)
    // ═══════════════════════════════════════════════════════════════════════

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
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Always use file scope - the agent will determine the actual scope from context
        let scope_def = ScopeDefinition::file(tag.file_path.clone());

        // Format target with line range if available
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

        self.jobs.insert(id, job);
        self.generation += 1;

        Ok(id)
    }

    /// Get all jobs for a specific workspace
    pub fn jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id))
            .collect()
    }

    /// Get pending jobs for a specific workspace
    pub fn pending_jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id) && j.status == JobStatus::Pending)
            .collect()
    }

    /// Get running jobs for a specific workspace
    pub fn running_jobs_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id == Some(workspace_id) && j.status == JobStatus::Running)
            .collect()
    }

    /// Get all jobs without a workspace (legacy jobs)
    pub fn jobs_without_workspace(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.workspace_id.is_none())
            .collect()
    }

    /// Associate an existing job with a workspace
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
