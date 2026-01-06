//! Job executor for the GUI
//!
//! Runs in a background thread and processes queued jobs

mod chain;
mod event;
mod git_utils;
mod log_forwarder;
mod run_job;
mod worktree_setup;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use crate::agent::AgentRegistry;
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{Job, JobStatus, LogEvent};

pub use event::ExecutorEvent;

/// Ensures file locks for a job are released on all exit paths.
pub(super) struct JobLockGuard {
    job_manager: Arc<Mutex<JobManager>>,
    job_id: crate::JobId,
}

impl JobLockGuard {
    pub(super) fn new(job_manager: Arc<Mutex<JobManager>>, job_id: crate::JobId) -> Self {
        Self { job_manager, job_id }
    }
}

impl Drop for JobLockGuard {
    fn drop(&mut self) {
        if let Ok(mut manager) = self.job_manager.lock() {
            manager.release_job_locks(self.job_id);
        }
    }
}

/// Start the job executor in a background thread
pub fn start_executor(
    work_dir: PathBuf,
    config: Arc<RwLock<Config>>,
    job_manager: Arc<Mutex<JobManager>>,
    event_tx: Sender<ExecutorEvent>,
    max_concurrent_jobs: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(executor_loop(
            work_dir,
            config,
            job_manager,
            event_tx,
            max_concurrent_jobs,
        ));
    });
}

async fn executor_loop(
    work_dir: PathBuf,
    config: Arc<RwLock<Config>>,
    job_manager: Arc<Mutex<JobManager>>,
    event_tx: Sender<ExecutorEvent>,
    max_concurrent_jobs: Arc<AtomicUsize>,
) {
    let agent_registry = AgentRegistry::new();
    let git_manager = GitManager::new(&work_dir).ok();

    // Cache config-derived values to reduce RwLock contention in the hot loop
    let mut cached_use_worktree = config
        .read()
        .map(|cfg| cfg.settings.use_worktree)
        .unwrap_or(false);
    let mut config_check_counter = 0u32;

    loop {
        // Only re-read config every 10 iterations (~5 seconds) to reduce lock contention
        config_check_counter += 1;
        if config_check_counter >= 10 {
            config_check_counter = 0;
            cached_use_worktree = config
                .read()
                .map(|cfg| cfg.settings.use_worktree)
                .unwrap_or(false);
        }
        let should_use_worktree = cached_use_worktree;

        let (_, queued_jobs) = {
            let Ok(mut manager) = job_manager.lock() else {
                // Lock poisoned - log and continue to next iteration
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(
                    "Job manager lock poisoned, skipping this tick".to_string(),
                )));
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            let running = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();

            let blocked_jobs: Vec<_> = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Blocked)
                .map(|j| (j.id, j.blocked_by, j.source_file.clone()))
                .collect();

            for (job_id, blocked_by, source_file) in blocked_jobs {
                if let Some(blocking_id) = blocked_by {
                    if manager
                        .get_blocking_job(&source_file, Some(job_id))
                        .is_none()
                    {
                        if let Some(j) = manager.get_mut(job_id) {
                            j.status = JobStatus::Queued;
                            j.blocked_by = None;
                            j.blocked_file = None;
                        }
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Job #{} unblocked (Job #{} finished)",
                            job_id, blocking_id
                        ))));
                    }
                }
            }

            // Calculate available slots and collect multiple queued jobs at once
            let max_jobs = max_concurrent_jobs.load(Ordering::Relaxed);
            let available = max_jobs.saturating_sub(running);

            let queued_jobs: Vec<Job> = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Queued)
                .take(available)
                .map(|j| (*j).clone())
                .collect();

            (running, queued_jobs)
        };

        if !queued_jobs.is_empty() {
            // Spawn all eligible jobs in parallel
            let mut spawn_handles = Vec::with_capacity(queued_jobs.len());

            for job in queued_jobs {
                let is_multi_agent = job.group_id.is_some();
                let needs_lock_check =
                    !should_use_worktree && !is_multi_agent && !job.force_worktree;

                if needs_lock_check {
                    let Ok(mut manager) = job_manager.lock() else {
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(
                            "Job manager lock poisoned, skipping job start".to_string(),
                        )));
                        continue;
                    };

                    if let Some(blocking_job_id) =
                        manager.get_blocking_job(&job.source_file, Some(job.id))
                    {
                        if let Some(j) = manager.get_mut(job.id) {
                            j.status = JobStatus::Blocked;
                            j.blocked_by = Some(blocking_job_id);
                            j.blocked_file = Some(job.source_file.clone());
                        }
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Job #{} blocked by Job #{} (file: {})",
                            job.id,
                            blocking_job_id,
                            job.source_file
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| job.source_file.display().to_string())
                        ))));
                        continue;
                    }

                    manager.try_lock_file(&job.source_file, job.id);
                }

                let work_dir = work_dir.clone();
                let config_snapshot = config.read().map(|cfg| cfg.clone()).unwrap_or_default();
                let job_manager = Arc::clone(&job_manager);
                let agent_registry = agent_registry.clone();
                let git_manager = git_manager.clone();
                let event_tx = event_tx.clone();

                spawn_handles.push(tokio::spawn(async move {
                    run_job::run_job(
                        &work_dir,
                        &config_snapshot,
                        &job_manager,
                        &agent_registry,
                        git_manager.as_ref(),
                        &event_tx,
                        job,
                    )
                    .await;
                }));
            }

            // All jobs spawned in parallel - handles are fire-and-forget
            // (they complete independently and update job status via job_manager)
            drop(spawn_handles);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
