//! Job manager implementation

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::git::find_git_root;
use crate::{CommentTag, Job, JobId, JobStatus, ScopeDefinition};

/// Manages job lifecycle (in-memory only, no persistence)
pub struct JobManager {
    /// Root directory of the repository
    #[allow(dead_code)]
    root: PathBuf,

    pub(super) jobs: HashMap<JobId, Job>,
    next_id: AtomicU64,

    /// File locks for concurrent job isolation
    file_locks: HashMap<PathBuf, JobId>,

    /// Generation counter - incremented on any mutation.
    /// Used by GUI to know when to refresh cached jobs.
    pub(super) generation: u64,
}

impl JobManager {
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

    /// Allocate the next job ID
    fn allocate_id(&self) -> JobId {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Insert a job and increment generation
    fn insert_job(&mut self, id: JobId, job: Job) {
        self.jobs.insert(id, job);
        self.generation += 1;
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

        // Determine workspace_path: git root > file's parent directory
        let workspace_path =
            find_git_root(&tag.file_path).or_else(|| tag.file_path.parent().map(PathBuf::from));

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

        job.workspace_path = workspace_path;
        self.insert_job(id, job);

        Ok(id)
    }

    pub fn get(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn get_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    pub fn jobs(&self) -> Vec<&Job> {
        self.jobs.values().collect()
    }

    pub fn pending_jobs(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .collect()
    }

    pub fn running_jobs(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::Running)
            .collect()
    }

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

    pub fn release_lock(&mut self, path: &Path) {
        self.file_locks.remove(path);
    }

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
}
