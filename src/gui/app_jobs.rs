//! Job management methods for KycoApp
//!
//! Contains job lifecycle operations: queue, apply, reject, kill, delete, etc.

use super::app::KycoApp;
use super::app_popup::ApplyTarget;
use super::jobs;
use crate::agent::bridge::PermissionMode;
use crate::{JobId, LogEvent, SdkType};

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

    /// Kill/stop a running job
    pub(crate) fn kill_job(&mut self, job_id: JobId) {
        let (agent_id, job_mode, session_id) = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.get(job_id) {
                Some(job) => (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                ),
                None => {
                    self.logs
                        .push(LogEvent::error(format!("Job #{} not found", job_id)));
                    return;
                }
            }
        };

        if let Some(session_id) = session_id.as_deref() {
            let sdk_type = self
                .config
                .read()
                .ok()
                .and_then(|cfg| cfg.get_agent_for_job(&agent_id, &job_mode))
                .map(|a| a.sdk_type)
                .unwrap_or_else(|| {
                    if agent_id == "codex" {
                        SdkType::Codex
                    } else {
                        SdkType::Claude
                    }
                });

            let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if sdk_type == SdkType::Codex {
                    self.bridge_client.interrupt_codex(session_id)
                } else {
                    self.bridge_client.interrupt_claude(session_id)
                }
            }));

            match interrupted {
                Ok(Ok(true)) => self.logs.push(LogEvent::system(format!(
                    "Sent interrupt for job #{}",
                    job_id
                ))),
                Ok(Ok(false)) => self.logs.push(LogEvent::error(format!(
                    "Interrupt was rejected (job #{})",
                    job_id
                ))),
                Ok(Err(e)) => self.logs.push(LogEvent::error(format!(
                    "Failed to interrupt job #{}: {}",
                    job_id, e
                ))),
                Err(_) => self.logs.push(LogEvent::error(format!(
                    "Bridge interrupt panicked (job #{})",
                    job_id
                ))),
            };
        }

        jobs::kill_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Set permission mode for a job's Claude session
    pub(crate) fn set_job_permission_mode(&mut self, job_id: JobId, mode: PermissionMode) {
        let (agent_id, job_mode, session_id) = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.get(job_id) {
                Some(job) => (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                ),
                None => {
                    self.logs
                        .push(LogEvent::error(format!("Job #{} not found", job_id)));
                    return;
                }
            }
        };

        let is_codex = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            config
                .get_agent_for_job(&agent_id, &job_mode)
                .map(|a| a.sdk_type == SdkType::Codex)
                .unwrap_or(agent_id == "codex")
        };

        if is_codex {
            self.logs.push(LogEvent::error(format!(
                "Permission mode switching is only supported for Claude sessions (job #{})",
                job_id
            )));
            return;
        }

        let Some(session_id) = session_id else {
            self.logs.push(LogEvent::error(format!(
                "Job #{} has no active Claude session yet",
                job_id
            )));
            return;
        };

        match self
            .bridge_client
            .set_claude_permission_mode(&session_id, mode)
        {
            Ok(true) => {
                self.permission_mode_overrides.insert(job_id, mode);
                self.logs.push(LogEvent::system(format!(
                    "Set permission mode to {} for job #{}",
                    match mode {
                        PermissionMode::Default => "default",
                        PermissionMode::AcceptEdits => "acceptEdits",
                        PermissionMode::BypassPermissions => "bypassPermissions",
                        PermissionMode::Plan => "plan",
                    },
                    job_id
                )));
            }
            Ok(false) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{} (bridge rejected request)",
                    job_id
                )));
            }
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{}: {}",
                    job_id, e
                )));
            }
        }
    }

    /// Mark a REPL job as complete
    pub(crate) fn mark_job_complete(&mut self, job_id: JobId) {
        jobs::mark_job_complete(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Continue a session job with a follow-up prompt
    pub(crate) fn continue_job_session(&mut self, job_id: JobId, prompt: String) {
        let (continuation_id, continuation_mode) = {
            let mut manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            let Some(original) = manager.get(job_id).cloned() else {
                self.logs
                    .push(LogEvent::error(format!("Job #{} not found", job_id)));
                return;
            };

            let Some(session_id) = original.bridge_session_id.clone() else {
                self.logs.push(LogEvent::error(format!(
                    "Job #{} has no session to continue",
                    job_id
                )));
                return;
            };

            let tag = crate::CommentTag {
                file_path: original.source_file.clone(),
                line_number: original.source_line,
                raw_line: format!("// @{}:{} {}", &original.agent_id, &original.mode, &prompt),
                agent: original.agent_id.clone(),
                agents: vec![original.agent_id.clone()],
                mode: original.mode.clone(),
                target: crate::Target::Block,
                status_marker: None,
                description: Some(prompt),
                job_id: None,
            };

            let continuation_id =
                match manager.create_job_with_range(&tag, &original.agent_id, None) {
                    Ok(id) => id,
                    Err(e) => {
                        self.logs.push(LogEvent::error(format!(
                            "Failed to create continuation job: {}",
                            e
                        )));
                        return;
                    }
                };

            if let Some(job) = manager.get_mut(continuation_id) {
                job.raw_tag_line = None;
                job.bridge_session_id = Some(session_id);

                // Reuse the same worktree and job context
                job.git_worktree_path = original.git_worktree_path.clone();
                job.branch_name = original.branch_name.clone();
                job.base_branch = original.base_branch.clone();
                job.scope = original.scope.clone();
                job.target = original.target;
                job.ide_context = original.ide_context;
                job.force_worktree = original.force_worktree;
                job.workspace_id = original.workspace_id;
                job.workspace_path = original.workspace_path.clone();
            }

            (continuation_id, original.mode)
        };

        self.logs.push(LogEvent::system(format!(
            "Created continuation job #{} (mode: {})",
            continuation_id, continuation_mode
        )));

        self.queue_job(continuation_id);
        self.selected_job_id = Some(continuation_id);
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
