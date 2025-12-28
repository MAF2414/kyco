//! Update loop helpers for KycoApp
//!
//! Contains event handling methods extracted from the main update loop.

use super::app::KycoApp;
use super::app_popup::ApplyTarget;
use super::app_types::ViewMode;
use super::executor::ExecutorEvent;
use super::groups::{ComparisonAction, render_comparison_popup};
use super::permission::{PermissionAction, PermissionRequest, render_permission_popup};
use super::update::{UpdateInfo, UpdateStatus};
use crate::LogEvent;
use crate::agent::bridge::{ToolApprovalResponse, ToolDecision};
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
                    let request = PermissionRequest {
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

    /// Handle voice events from VoiceManager
    pub(crate) fn handle_voice_events(&mut self, ctx: &egui::Context) {
        for event in self.voice_manager.poll_events() {
            match event {
                super::voice::VoiceEvent::TranscriptionComplete { text, from_manual } => {
                    // Check if this was a global voice recording (from hotkey)
                    if self.global_voice_recording {
                        // Global voice input - auto-paste to focused application
                        self.handle_global_voice_transcription(&text);
                    } else if from_manual {
                        // Manual recording (button press in popup) - just append text, no wakeword detection
                        if self.popup_input.is_empty() {
                            self.popup_input = text;
                        } else {
                            self.popup_input.push(' ');
                            self.popup_input.push_str(&text);
                        }
                        self.update_suggestions();
                        self.logs
                            .push(LogEvent::system("Voice transcription complete".to_string()));

                        // Auto-execute if Enter was pressed during recording
                        if self.voice_pending_execute {
                            self.voice_pending_execute = false;
                            self.execute_popup_task(false); // Normal execution (no force worktree)
                        }
                    } else {
                        // Continuous listening - try wakeword detection
                        if let Some(wakeword_match) =
                            self.voice_manager.config.action_registry.match_text(&text)
                        {
                            // Wakeword matched - use mode and prompt from the match
                            self.popup_input = format!(
                                "{} {}",
                                wakeword_match.mode,
                                wakeword_match.get_final_prompt()
                            );
                            self.update_suggestions();

                            // Open selection popup if not already open
                            if self.view_mode != ViewMode::SelectionPopup {
                                self.view_mode = ViewMode::SelectionPopup;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                            }

                            self.logs.push(LogEvent::system(format!(
                                "Voice wakeword '{}' → mode '{}'",
                                wakeword_match.wakeword, wakeword_match.mode
                            )));
                        } else {
                            // No wakeword match - fall back to legacy keyword parsing
                            let (detected_mode, prompt) = super::voice::parse_voice_input(
                                &text,
                                &self.voice_manager.config.keywords,
                            );

                            // Update input field with transcribed text
                            if let Some(mode) = detected_mode {
                                self.popup_input = format!("{} {}", mode, prompt);
                            } else {
                                // If no mode detected, append to existing input
                                if self.popup_input.is_empty() {
                                    self.popup_input = text;
                                } else {
                                    self.popup_input.push(' ');
                                    self.popup_input.push_str(&text);
                                }
                            }
                            self.update_suggestions();
                            self.logs
                                .push(LogEvent::system("Voice transcription complete".to_string()));
                        }
                    }
                }
                super::voice::VoiceEvent::WakewordMatched {
                    wakeword,
                    mode,
                    prompt,
                } => {
                    // Direct wakeword match from continuous listening
                    self.popup_input = format!("{} {}", mode, prompt);
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!(
                        "Voice wakeword: {} → {}",
                        wakeword, mode
                    )));
                }
                super::voice::VoiceEvent::KeywordDetected { keyword, full_text } => {
                    // In continuous mode: keyword detected, trigger hotkey and fill input
                    self.popup_input = format!(
                        "{} {}",
                        keyword,
                        full_text.trim_start_matches(&keyword).trim()
                    );
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!(
                        "Voice keyword detected: {}",
                        keyword
                    )));
                }
                super::voice::VoiceEvent::Error { message } => {
                    self.logs
                        .push(LogEvent::error(format!("Voice error: {}", message)));
                    self.voice_pending_execute = false; // Cancel pending execution on error
                    // Reset global voice state on error
                    if self.global_voice_recording {
                        self.global_voice_recording = false;
                        self.show_voice_overlay = false;
                    }
                }
                super::voice::VoiceEvent::RecordingStarted => {
                    self.logs
                        .push(LogEvent::system("Voice recording started".to_string()));
                }
                super::voice::VoiceEvent::RecordingStopped { duration_secs } => {
                    self.logs.push(LogEvent::system(format!(
                        "Voice recording stopped ({:.1}s)",
                        duration_secs
                    )));
                }
                super::voice::VoiceEvent::VadSpeechStarted => {
                    self.logs
                        .push(LogEvent::system("VAD: Speech detected".to_string()));
                }
                super::voice::VoiceEvent::VadSpeechEnded => {
                    self.logs
                        .push(LogEvent::system("VAD: Speech ended".to_string()));
                }
                _ => {}
            }
        }
    }

    /// Poll async voice installation progress (non-blocking)
    pub(crate) fn poll_voice_install_progress(&mut self) {
        if let Some(handle) = &self.voice_install_handle {
            while let Ok(progress) = handle.progress_rx.try_recv() {
                use super::voice::install::InstallProgress;
                match progress {
                    InstallProgress::Step {
                        step,
                        total,
                        message,
                    } => {
                        self.voice_install_status = Some((
                            format!(
                                "Installing voice dependencies ({}/{})...\n{}",
                                step, total, message
                            ),
                            false,
                        ));
                        self.logs.push(LogEvent::system(format!(
                            "Voice install step {}/{}: {}",
                            step, total, message
                        )));
                    }
                    InstallProgress::Complete(result) => {
                        self.voice_install_status = Some((result.message.clone(), result.is_error));
                        self.voice_install_in_progress = false;
                        self.voice_install_handle = None;
                        self.logs.push(LogEvent::system(format!(
                            "Voice installation complete: {}",
                            result.message
                        )));
                        // Invalidate availability cache so next check sees the new installation
                        self.voice_manager.reset();
                        break; // Handle is now None, exit loop
                    }
                    InstallProgress::Failed(result) => {
                        self.voice_install_status = Some((result.message.clone(), true));
                        self.voice_install_in_progress = false;
                        self.voice_install_handle = None;
                        self.logs.push(LogEvent::error(format!(
                            "Voice installation failed: {}",
                            result.message
                        )));
                        break; // Handle is now None, exit loop
                    }
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

    /// Render the permission popup modal (on top of everything)
    pub(crate) fn render_permission_popup_modal(&mut self, ctx: &egui::Context) {
        if let Some(action) = render_permission_popup(ctx, &mut self.permission_state) {
            match action {
                PermissionAction::Approve(request_id) => {
                    // Send approval to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Allow,
                        reason: None,
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(true) => {
                            self.logs.push(LogEvent::system(format!(
                                "✓ Approved tool request: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Ok(false) => {
                            self.logs.push(LogEvent::error(format!(
                                "Tool approval rejected by bridge: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool approval: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::ApproveAll(request_ids) => {
                    let mut approved = 0usize;
                    for request_id in &request_ids {
                        let response = ToolApprovalResponse {
                            request_id: request_id.clone(),
                            decision: ToolDecision::Allow,
                            reason: None,
                            modified_input: None,
                        };
                        match self.bridge_client.send_tool_approval(&response) {
                            Ok(true) => {
                                approved += 1;
                            }
                            Ok(false) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Tool approval rejected by bridge: {}",
                                    &request_id[..12.min(request_id.len())]
                                )));
                            }
                            Err(e) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Failed to send tool approval: {}",
                                    e
                                )));
                            }
                        }
                    }

                    self.logs.push(LogEvent::system(format!(
                        "✓ Approved {} tool request(s)",
                        approved
                    )));

                    // Clear popup state
                    self.permission_state.current_request = None;
                    self.permission_state.pending_requests.clear();
                    self.permission_state.visible = false;
                    self.permission_state.should_focus = false;
                }
                PermissionAction::Deny(request_id, reason) => {
                    // Send denial to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some(reason.clone()),
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(_) => {
                            self.logs.push(LogEvent::system(format!(
                                "✗ Denied tool request: {} ({})",
                                &request_id[..12.min(request_id.len())],
                                reason
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool denial: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::Dismiss(request_id) => {
                    // Treat dismiss as deny
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some("User dismissed".to_string()),
                        modified_input: None,
                    };
                    let _ = self.bridge_client.send_tool_approval(&response);
                    self.logs.push(LogEvent::system(format!(
                        "Dismissed tool request: {}",
                        &request_id[..12.min(request_id.len())]
                    )));
                    self.permission_state.next_request();
                }
            }
        }

        // Bring app to foreground if needed
        if self.permission_state.should_focus {
            self.permission_state.should_focus = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
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
