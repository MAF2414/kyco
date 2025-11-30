//! Job manager implementation

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
        }
    }

    /// Create a new job manager (for API compatibility, same as new())
    pub fn load(root: &Path) -> Result<Self> {
        Ok(Self::new(root))
    }

    /// Create a new job from a comment tag
    pub fn create_job(&mut self, tag: &CommentTag, agent_id: &str) -> Result<JobId> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Always use file scope - the agent will determine the actual scope from context
        let scope_def = ScopeDefinition::file(tag.file_path.clone());

        let target = format!("{}:{}", tag.file_path.display(), tag.line_number);

        let job = Job::new(
            id,
            tag.mode.clone(),
            scope_def,
            target,
            tag.description.clone(),
            agent_id.to_string(),
            tag.file_path.clone(),
            tag.line_number,
            Some(tag.raw_line.clone()),
        );

        self.jobs.insert(id, job);

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
        }
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
}
