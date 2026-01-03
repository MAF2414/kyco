//! Job management methods for KycoApp
//!
//! Contains job lifecycle operations: queue, apply, reject, kill, delete, etc.

mod lifecycle;
mod session;

use super::app::KycoApp;
use super::app_popup::ApplyTarget;
use super::jobs;
use crate::{CommentTag, JobId, JobStatus, LogEvent, Target};

impl KycoApp {
    /// Queue a job for execution
    pub(crate) fn queue_job(&mut self, job_id: JobId) {
        jobs::queue_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Apply job changes (merge worktree to main)
    pub(crate) fn apply_job(&mut self, job_id: JobId) {
        let job = match self.job_manager.lock() {
            Ok(manager) => manager.get(job_id).cloned(),
            Err(_) => None,
        };

        let Some(job) = job else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        let target = if let Some(group_id) = job.group_id {
            ApplyTarget::Group {
                group_id,
                selected_job_id: job_id,
            }
        } else {
            ApplyTarget::Single { job_id }
        };

        self.open_apply_confirm(target);
    }

    /// Reject job changes
    pub(crate) fn reject_job(&mut self, job_id: JobId) {
        let job = match self.job_manager.lock() {
            Ok(manager) => manager.get(job_id).cloned(),
            Err(_) => None,
        };

        let Some(job) = job else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        if let Some(worktree) = job.git_worktree_path.clone() {
            let workspace_root = self.workspace_root_for_job(&job);
            if let Ok(git) = crate::git::GitManager::new(&workspace_root) {
                if let Err(e) = git.remove_worktree_by_path(&worktree) {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to remove worktree for rejected job: {}",
                        e
                    )));
                }
            } else {
                self.logs.push(LogEvent::error(format!(
                    "Failed to initialize git manager for {}",
                    workspace_root.display()
                )));
            }
        } else {
            self.logs.push(LogEvent::system(
                "Rejected job without worktree (no changes were reverted)".to_string(),
            ));
        }

        if let Ok(mut manager) = self.job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.set_status(crate::JobStatus::Rejected);
                j.git_worktree_path = None;
                j.branch_name = None;
            }
        }
        self.logs
            .push(LogEvent::system(format!("Rejected job #{}", job_id)));
        self.refresh_jobs();
    }

    /// Mark a REPL job as complete
    pub(crate) fn mark_job_complete(&mut self, job_id: JobId) {
        jobs::mark_job_complete(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Check if a job's completion means a group is ready for comparison
    pub(crate) fn check_group_completion(&mut self, job_id: JobId) {
        // Get the group ID for this job
        let group_id = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            match manager.get(job_id) {
                Some(job) => job.group_id,
                None => return,
            }
        };

        let group_id = match group_id {
            Some(id) => id,
            None => return, // Job is not part of a group
        };

        // Collect job references for status update
        let jobs: Vec<&crate::Job> = self.cached_jobs.iter().collect();

        // Update group status
        if let Ok(mut gm) = self.group_manager.lock() {
            gm.update_group_status(group_id, &jobs);

            // Check if group is now in Comparing status
            if let Some(group) = gm.get(group_id) {
                if group.status == crate::GroupStatus::Comparing {
                    // Log that the group is ready
                    self.logs.push(LogEvent::system(format!(
                        "Group #{} ready for comparison ({} agents)",
                        group_id,
                        group.job_ids.len()
                    )));
                }
            }
        }
    }

    /// Delete a job from the job manager
    pub(crate) fn delete_job(&mut self, job_id: JobId) {
        if let Ok(mut manager) = self.job_manager.lock() {
            if let Some(job) = manager.remove_job(job_id) {
                self.logs.push(LogEvent::system(format!(
                    "Deleted job #{} ({})",
                    job_id, job.mode
                )));

                // Clear selection if deleted job was selected
                if self.selected_job_id == Some(job_id) {
                    self.selected_job_id = None;
                }
            }
        }

        // Also remove from group manager
        if let Ok(mut gm) = self.group_manager.lock() {
            gm.remove_job(job_id);
        }

        // Cleanup per-job UI state
        self.permission_mode_overrides.remove(&job_id);

        // Refresh to update UI
        self.refresh_jobs();
    }

    /// Restart a failed or rejected job with the same parameters
    pub(crate) fn restart_job(&mut self, job_id: JobId) {
        let original = match self.job_manager.lock() {
            Ok(manager) => manager.get(job_id).cloned(),
            Err(_) => {
                self.logs
                    .push(LogEvent::error("Failed to lock job manager"));
                return;
            }
        };

        let Some(original) = original else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        // Only allow restart for failed or rejected jobs
        if !matches!(original.status, JobStatus::Failed | JobStatus::Rejected) {
            self.logs.push(LogEvent::error(format!(
                "Job #{} cannot be restarted (status: {})",
                job_id, original.status
            )));
            return;
        }

        let description = original.description.clone().unwrap_or_default();

        let tag = CommentTag {
            file_path: original.source_file.clone(),
            line_number: original.source_line,
            raw_line: String::new(),
            agent: original.agent_id.clone(),
            agents: vec![original.agent_id.clone()],
            mode: original.mode.clone(),
            target: Target::Block,
            status_marker: None,
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            job_id: None,
        };

        let new_job_id = {
            let mut manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.create_job_with_range(&tag, &original.agent_id, None)
            {
                Ok(id) => {
                    // Copy relevant context from original job
                    if let Some(job) = manager.get_mut(id) {
                        job.raw_tag_line = None;
                        job.ide_context = original.ide_context.clone();
                        job.force_worktree = original.force_worktree;
                        job.workspace_path = original.workspace_path.clone();
                        job.scope = original.scope.clone();
                        job.target = original.target;
                    }
                    id
                }
                Err(e) => {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to create restart job: {}",
                        e
                    )));
                    return;
                }
            }
        };

        // Queue the new job immediately
        jobs::queue_job(&self.job_manager, new_job_id, &mut self.logs);

        self.logs.push(LogEvent::system(format!(
            "Restarted job #{} as #{}",
            job_id, new_job_id
        )));

        // Select the new job
        self.selected_job_id = Some(new_job_id);

        self.refresh_jobs();
    }

    /// Delete all finished jobs (Done, Failed, Rejected, Merged)
    pub(crate) fn delete_all_finished_jobs(&mut self) {
        // Collect IDs of finished jobs
        let finished_ids: Vec<JobId> = self
            .cached_jobs
            .iter()
            .filter(|j| j.is_finished())
            .map(|j| j.id)
            .collect();

        if finished_ids.is_empty() {
            return;
        }

        let count = finished_ids.len();

        // Remove from job manager
        if let Ok(mut manager) = self.job_manager.lock() {
            for job_id in &finished_ids {
                manager.remove_job(*job_id);
            }
        }

        // Remove from group manager
        if let Ok(mut gm) = self.group_manager.lock() {
            for job_id in &finished_ids {
                gm.remove_job(*job_id);
            }
        }

        // Cleanup per-job UI state
        for job_id in &finished_ids {
            self.permission_mode_overrides.remove(job_id);
        }

        // Clear selection if deleted job was selected
        if let Some(selected) = self.selected_job_id {
            if finished_ids.contains(&selected) {
                self.selected_job_id = None;
            }
        }

        self.logs
            .push(LogEvent::system(format!("Deleted {} finished jobs", count)));

        // Refresh to update UI
        self.refresh_jobs();
    }
}
