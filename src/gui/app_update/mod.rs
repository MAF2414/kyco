//! Update loop helpers for KycoApp
//!
//! Contains event handling methods extracted from the main update loop.

mod permission;
mod voice;

use super::app::KycoApp;
use super::app_popup::ApplyTarget;
use super::app_types::ViewMode;
use super::executor::ExecutorEvent;
use super::groups::{ComparisonAction, render_comparison_popup};
use super::update::{UpdateInfo, UpdateStatus};
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    /// Handle executor events (job status updates, logs, permission requests)
    pub(crate) fn handle_executor_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.executor_rx.try_recv() {
            match event {
                ExecutorEvent::JobStarted(job_id) => {
                    self.logs
                        .push(LogEvent::system(format!("Job #{} started", job_id)));
                }
                ExecutorEvent::JobCompleted(job_id) => {
                    self.logs
                        .push(LogEvent::system(format!("Job #{} completed", job_id)));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                    // Reload diff if this is the currently selected job
                    if self.selected_job_id == Some(job_id) {
                        self.load_inline_diff_for_selected();
                    }
                }
                ExecutorEvent::JobFailed(job_id, error) => {
                    self.logs.push(LogEvent::error(format!(
                        "Job #{} failed: {}",
                        job_id, error
                    )));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                }
                ExecutorEvent::ChainStepCompleted {
                    job_id,
                    step_index,
                    total_steps,
                    mode,
                    state,
                    step_summary,
                } => {
                    let state_str = state.as_deref().unwrap_or("none");
                    self.logs.push(LogEvent::system(format!(
                        "Chain step {}/{} completed: {} (state: {})",
                        step_index + 1,
                        total_steps,
                        mode,
                        state_str
                    )));
                    // Update chain progress in the job for real-time display
                    if let Ok(mut manager) = self.job_manager.lock() {
                        if let Some(job) = manager.get_mut(job_id) {
                            job.chain_current_step = Some(step_index + 1);
                            // Add step to history if not already present
                            if job.chain_step_history.len() <= step_index {
                                job.chain_step_history.push(step_summary);
                            }
                        }
                    }
                }
                ExecutorEvent::ChainCompleted {
                    job_id: _,
                    chain_name,
                    steps_executed,
                    success,
                } => {
                    if success {
                        self.logs.push(LogEvent::system(format!(
                            "Chain '{}' completed: {} steps executed",
                            chain_name, steps_executed
                        )));
                    } else {
                        self.logs.push(LogEvent::error(format!(
                            "Chain '{}' failed after {} steps",
                            chain_name, steps_executed
                        )));
                    }
                }
                ExecutorEvent::Log(log_event) => {
                    self.logs.push(log_event);
                }
                ExecutorEvent::PermissionNeeded {
                    job_id,
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                } => {
                    // Convert to PermissionRequest and add to popup queue
                    let request = super::permission::PermissionRequest {
                        request_id,
                        session_id,
                        tool_name: tool_name.clone(),
                        tool_input,
                        received_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    };
                    self.permission_state.add_request(request);
                    self.logs.push(
                        LogEvent::permission(format!(
                            "Permission request: {} (waiting)",
                            tool_name
                        ))
                        .for_job(job_id),
                    );

                    // Bring window to front so user notices the permission request
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
        }
    }

    /// Poll apply/merge result if an operation is running
    pub(crate) fn poll_apply_result(&mut self) {
        let apply_result = self
            .apply_confirm_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());

        if let Some(result) = apply_result {
            self.apply_confirm_rx = None;

            match result {
                Ok(outcome) => {
                    match outcome.target {
                        ApplyTarget::Single { job_id } => {
                            if let Ok(mut jm) = self.job_manager.lock() {
                                if let Some(job) = jm.get_mut(job_id) {
                                    job.set_status(crate::JobStatus::Merged);
                                    job.git_worktree_path = None;
                                    job.branch_name = None;
                                }
                            }
                        }
                        ApplyTarget::Group {
                            group_id,
                            selected_job_id,
                        } => {
                            if let Ok(mut gm) = self.group_manager.lock() {
                                gm.select_result(group_id, selected_job_id);
                                gm.mark_merged(group_id);
                            }

                            if let Ok(mut jm) = self.job_manager.lock() {
                                for job_id in &outcome.group_job_ids {
                                    if let Some(job) = jm.get_mut(*job_id) {
                                        if *job_id == selected_job_id {
                                            job.set_status(crate::JobStatus::Merged);
                                        } else {
                                            job.set_status(crate::JobStatus::Rejected);
                                        }
                                        job.git_worktree_path = None;
                                        job.branch_name = None;
                                    }
                                }
                            }

                            self.comparison_state.close();
                        }
                    }

                    self.logs.push(LogEvent::system(outcome.message));
                    self.apply_confirm_target = None;
                    self.apply_confirm_error = None;
                    self.view_mode = ViewMode::JobList;
                    self.refresh_jobs();
                }
                Err(err) => {
                    self.apply_confirm_error = Some(err);
                }
            }
        }
    }

    /// Auto-queue pending jobs when auto_run is enabled
    pub(crate) fn auto_queue_pending_jobs(&mut self) {
        if self.auto_run {
            let pending_job_ids: Vec<u64> = self
                .cached_jobs
                .iter()
                .filter(|j| j.status == crate::JobStatus::Pending)
                .map(|j| j.id)
                .collect();

            if !pending_job_ids.is_empty() {
                if let Ok(mut manager) = self.job_manager.lock() {
                    for job_id in pending_job_ids {
                        manager.set_status(job_id, crate::JobStatus::Queued);
                        self.logs.push(crate::LogEvent::system(format!(
                            "Auto-queued job #{}",
                            job_id
                        )));
                    }
                }
            }
        }
    }

    /// Render the comparison popup for multi-agent results
    pub(crate) fn render_comparison_popup(&mut self, ctx: &egui::Context) {
        if let Some(action) = render_comparison_popup(ctx, &mut self.comparison_state) {
            match action {
                ComparisonAction::SelectJob(job_id) => {
                    // Update the selection in the group manager
                    if let Some(group_id) = self.comparison_state.group_id() {
                        if let Ok(mut gm) = self.group_manager.lock() {
                            gm.select_result(group_id, job_id);
                        }
                    }
                    self.logs
                        .push(LogEvent::system(format!("Selected job #{}", job_id)));
                }
                ComparisonAction::ViewDiff(job_id) => {
                    self.open_job_diff(job_id, ViewMode::ComparisonPopup);
                }
                ComparisonAction::MergeAndClose => {
                    if let Some(group_id) = self.comparison_state.group_id() {
                        let Some(selected_job_id) = self.comparison_state.selected_job_id else {
                            self.logs
                                .push(LogEvent::error("No job selected for merge".to_string()));
                            return;
                        };

                        self.open_apply_confirm(ApplyTarget::Group {
                            group_id,
                            selected_job_id,
                        });
                    }
                }
                ComparisonAction::Cancel => {
                    // Close popup without merging
                    self.comparison_state.close();
                    self.view_mode = ViewMode::JobList;
                }
            }
        }
    }

    /// Poll the update checker and return update info if available.
    pub(crate) fn poll_update_checker(&mut self) -> Option<UpdateInfo> {
        match self.update_checker.poll() {
            UpdateStatus::UpdateAvailable(info) => Some(info.clone()),
            _ => None,
        }
    }

    /// Handle update install request if pending, and poll for install results.
    /// Returns true if an install was started or is in progress.
    pub(crate) fn handle_update_install(&mut self, update_info: Option<&UpdateInfo>) {
        // Handle install request
        if matches!(
            self.update_install_status,
            super::status_bar::InstallStatus::InstallRequested
        ) {
            if let Some(info) = update_info {
                self.update_install_status = super::status_bar::InstallStatus::Installing;
                let (tx, rx) = std::sync::mpsc::channel();
                self.update_install_rx = Some(rx);
                let info_clone = info.clone();
                std::thread::spawn(move || {
                    let result = super::update::install_update(&info_clone);
                    let _ = tx.send(result);
                });
            }
        }

        // Poll install result if we're installing
        if let Some(rx) = &self.update_install_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(msg) => {
                        self.update_install_status = super::status_bar::InstallStatus::Success(msg)
                    }
                    Err(err) => {
                        self.update_install_status = super::status_bar::InstallStatus::Error(err)
                    }
                }
                self.update_install_rx = None;
            }
        }
    }
}
