//! Helper methods for KycoApp
//!
//! Memory management and state manipulation methods.

use super::app::KycoApp;
use super::app_popup::ApplyTarget;
use super::app_types::ViewMode;
use super::jobs;
use crate::{AgentGroupId, Job};
use std::path::PathBuf;

/// Maximum number of log entries to keep in memory (FIFO eviction)
const MAX_GLOBAL_LOGS: usize = 500;

impl KycoApp {
    // ═══════════════════════════════════════════════════════════════════════
    // Memory Management Helpers
    // ═══════════════════════════════════════════════════════════════════════

    /// Truncate global logs to MAX_GLOBAL_LOGS (FIFO eviction).
    /// Called periodically to prevent unbounded memory growth.
    pub(crate) fn truncate_logs(&mut self) {
        if self.logs.len() > MAX_GLOBAL_LOGS {
            let excess = self.logs.len() - MAX_GLOBAL_LOGS;
            self.logs.drain(0..excess);
        }
    }

    /// Refresh cached jobs from JobManager (only if changed)
    pub(crate) fn refresh_jobs(&mut self) {
        // Only refresh if jobs have changed (generation counter check)
        if let Some(new_generation) =
            jobs::check_jobs_changed(&self.job_manager, self.last_job_generation)
        {
            let (new_jobs, generation) = jobs::refresh_jobs(&self.job_manager);
            self.cached_jobs = new_jobs;
            self.last_job_generation = generation;
            tracing::trace!(
                "Jobs refreshed, generation {} -> {}",
                self.last_job_generation,
                new_generation
            );
        }
    }

    pub(crate) fn open_apply_confirm(&mut self, target: ApplyTarget) {
        self.apply_confirm_target = Some(target);
        self.apply_confirm_return_view = self.view_mode;
        self.apply_confirm_error = None;
        self.apply_confirm_rx = None;
        self.view_mode = ViewMode::ApplyConfirmPopup;
    }

    pub(crate) fn workspace_root_for_job(&self, job: &Job) -> PathBuf {
        job.workspace_path
            .clone()
            .unwrap_or_else(|| self.work_dir.clone())
    }

    /// Open the comparison popup for a group
    pub(crate) fn open_comparison_popup(&mut self, group_id: AgentGroupId) {
        // Get the group
        let group = {
            let gm = match self.group_manager.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            match gm.get(group_id) {
                Some(g) => g.clone(),
                None => return,
            }
        };

        // Collect jobs for this group
        let jobs: Vec<Job> = self
            .cached_jobs
            .iter()
            .filter(|j| j.group_id == Some(group_id))
            .cloned()
            .collect();

        // Open the popup
        self.comparison_state.open(group, jobs);
        self.view_mode = ViewMode::ComparisonPopup;
    }

    /// Write a job request file
    #[allow(dead_code)]
    pub(crate) fn write_job_request(
        &self,
        agent: &str,
        mode: &str,
        prompt: &str,
    ) -> std::io::Result<()> {
        jobs::write_job_request(&self.work_dir, &self.selection, agent, mode, prompt)
    }
}
