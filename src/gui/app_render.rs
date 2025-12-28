//! Render delegation methods for KycoApp
//!
//! Contains methods that delegate to specialized render modules.

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::jobs;
use super::theme::BG_PRIMARY;
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    /// Render settings/extensions view
    pub(crate) fn render_settings(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            // Lock poisoned - show error and return
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render settings",
            ));
            return;
        };
        super::settings::render_settings(
            ctx,
            &mut super::settings::SettingsState {
                // General settings
                settings_max_concurrent: &mut self.settings_max_concurrent,
                settings_auto_run: &mut self.settings_auto_run,
                settings_use_worktree: &mut self.settings_use_worktree,
                settings_output_schema: &mut self.settings_output_schema,
                settings_structured_output_schema: &mut self.settings_structured_output_schema,
                settings_status: &mut self.settings_status,
                // Voice settings
                voice_settings_mode: &mut self.voice_settings_mode,
                voice_settings_keywords: &mut self.voice_settings_keywords,
                voice_settings_model: &mut self.voice_settings_model,
                voice_settings_language: &mut self.voice_settings_language,
                voice_settings_silence_threshold: &mut self.voice_settings_silence_threshold,
                voice_settings_silence_duration: &mut self.voice_settings_silence_duration,
                voice_settings_max_duration: &mut self.voice_settings_max_duration,
                voice_settings_global_hotkey: &mut self.voice_settings_global_hotkey,
                voice_settings_popup_hotkey: &mut self.voice_settings_popup_hotkey,
                voice_install_status: &mut self.voice_install_status,
                voice_install_in_progress: &mut self.voice_install_in_progress,
                voice_install_handle: &mut self.voice_install_handle,
                // Voice test state
                voice_test_status: &mut self.voice_test_status,
                voice_test_result: &mut self.voice_test_result,
                // VAD settings
                vad_enabled: &mut self.vad_enabled,
                vad_speech_threshold: &mut self.vad_speech_threshold,
                vad_silence_duration_ms: &mut self.vad_silence_duration_ms,
                // Voice action registry (from voice manager config)
                voice_action_registry: &self.voice_manager.config.action_registry,
                // Extension status
                extension_status: &mut self.extension_status,
                // Navigation and config
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
                // Voice config change tracking
                voice_config_changed: &mut self.voice_config_changed,
                // Shared max concurrent jobs (for runtime updates to executor)
                max_concurrent_jobs_shared: &self.max_concurrent_jobs,
                // Workspace config import
                workspace_registry: Some(&self.workspace_registry),
                import_workspace_selected: &mut self.import_workspace_selected,
                import_modes: &mut self.import_modes,
                import_agents: &mut self.import_agents,
                import_chains: &mut self.import_chains,
                import_settings: &mut self.import_settings,
                orchestrator_cli_command: &mut self.orchestrator_cli_command,
                orchestrator_system_prompt: &mut self.orchestrator_system_prompt,
            },
        );
    }

    /// Render modes configuration view
    pub(crate) fn render_modes(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs
                .push(LogEvent::error("Config lock poisoned, cannot render modes"));
            return;
        };
        super::modes::render_modes(
            ctx,
            &mut super::modes::ModeEditorState {
                selected_mode: &mut self.selected_mode,
                mode_edit_name: &mut self.mode_edit_name,
                mode_edit_aliases: &mut self.mode_edit_aliases,
                mode_edit_prompt: &mut self.mode_edit_prompt,
                mode_edit_system_prompt: &mut self.mode_edit_system_prompt,
                mode_edit_readonly: &mut self.mode_edit_readonly,
                mode_edit_status: &mut self.mode_edit_status,
                mode_edit_agent: &mut self.mode_edit_agent,
                mode_edit_allowed_tools: &mut self.mode_edit_allowed_tools,
                mode_edit_disallowed_tools: &mut self.mode_edit_disallowed_tools,
                mode_edit_session_mode: &mut self.mode_edit_session_mode,
                mode_edit_max_turns: &mut self.mode_edit_max_turns,
                mode_edit_model: &mut self.mode_edit_model,
                mode_edit_claude_permission: &mut self.mode_edit_claude_permission,
                mode_edit_codex_sandbox: &mut self.mode_edit_codex_sandbox,
                mode_edit_output_states: &mut self.mode_edit_output_states,
                mode_edit_state_prompt: &mut self.mode_edit_state_prompt,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render agents configuration view
    pub(crate) fn render_agents(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render agents",
            ));
            return;
        };
        super::agents::render_agents(
            ctx,
            &mut super::agents::AgentEditorState {
                selected_agent: &mut self.selected_agent,
                agent_edit_name: &mut self.agent_edit_name,
                agent_edit_aliases: &mut self.agent_edit_aliases,
                agent_edit_cli_type: &mut self.agent_edit_cli_type,
                agent_edit_mode: &mut self.agent_edit_mode,
                agent_edit_system_prompt_mode: &mut self.agent_edit_system_prompt_mode,
                agent_edit_disallowed_tools: &mut self.agent_edit_disallowed_tools,
                agent_edit_allowed_tools: &mut self.agent_edit_allowed_tools,
                agent_edit_status: &mut self.agent_edit_status,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render chains configuration view
    pub(crate) fn render_chains(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render chains",
            ));
            return;
        };
        super::chains::render_chains(
            ctx,
            &mut super::chains::ChainEditorState {
                selected_chain: &mut self.selected_chain,
                chain_edit_name: &mut self.chain_edit_name,
                chain_edit_description: &mut self.chain_edit_description,
                chain_edit_states: &mut self.chain_edit_states,
                chain_edit_steps: &mut self.chain_edit_steps,
                chain_edit_stop_on_failure: &mut self.chain_edit_stop_on_failure,
                chain_edit_pass_full_response: &mut self.chain_edit_pass_full_response,
                chain_edit_status: &mut self.chain_edit_status,
                pending_confirmation: &mut self.chain_pending_confirmation,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render the main content based on current view mode
    pub(crate) fn render_view_mode(&mut self, ctx: &egui::Context) {
        match self.view_mode {
            ViewMode::JobList => {
                egui::SidePanel::left("job_list")
                    .default_width(280.0)
                    .min_width(280.0)
                    .max_width(280.0)
                    .resizable(false)
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_job_list(ui);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_detail_panel(ui);
                    });
            }
            ViewMode::SelectionPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_selection_popup(ctx);
            }
            ViewMode::BatchPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_batch_popup(ctx);
            }
            ViewMode::DiffView => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_diff_popup(ctx);
            }
            ViewMode::ApplyConfirmPopup => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_apply_confirm_popup(ctx);
            }
            ViewMode::ComparisonPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_comparison_popup(ctx);
            }
            ViewMode::Settings => {
                self.render_settings(ctx);
                // Apply voice config changes to VoiceManager after settings are saved
                if self.voice_config_changed {
                    self.voice_config_changed = false;
                    self.apply_voice_config();
                }
            }
            ViewMode::Modes => {
                self.render_modes(ctx);
            }
            ViewMode::Agents => {
                self.render_agents(ctx);
            }
            ViewMode::Chains => {
                self.render_chains(ctx);
            }
        }
    }

    /// Render the job list panel
    pub(crate) fn render_job_list(&mut self, ui: &mut egui::Ui) {
        let action = jobs::render_job_list(
            ui,
            &self.cached_jobs,
            &mut self.selected_job_id,
            &mut self.job_list_filter,
        );

        // Handle actions
        match action {
            jobs::JobListAction::DeleteJob(job_id) => {
                self.delete_job(job_id);
            }
            jobs::JobListAction::DeleteAllFinished => {
                self.delete_all_finished_jobs();
            }
            jobs::JobListAction::None => {}
        }
    }

    /// Render the detail panel
    pub(crate) fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        use super::detail_panel::{DetailPanelAction, DetailPanelState, render_detail_panel};

        let action = {
            let Ok(config) = self.config.read() else {
                ui.label("Config unavailable");
                return;
            };
            let mut state = DetailPanelState {
                selected_job_id: self.selected_job_id,
                cached_jobs: &self.cached_jobs,
                logs: &self.logs,
                config: &config,
                log_scroll_to_bottom: self.log_scroll_to_bottom,
                activity_log_filters: &mut self.activity_log_filters,
                continuation_prompt: &mut self.continuation_prompt,
                commonmark_cache: &mut self.commonmark_cache,
                permission_mode_overrides: &self.permission_mode_overrides,
                diff_content: self.inline_diff_content.as_deref(),
            };

            render_detail_panel(ui, &mut state)
        };

        if let Some(action) = action {
            match action {
                DetailPanelAction::Queue(job_id) => self.queue_job(job_id),
                DetailPanelAction::Apply(job_id) => self.apply_job(job_id),
                DetailPanelAction::Reject(job_id) => self.reject_job(job_id),
                DetailPanelAction::CompareGroup(group_id) => self.open_comparison_popup(group_id),
                DetailPanelAction::Continue(job_id, prompt) => {
                    self.continue_job_session(job_id, prompt);
                }
                DetailPanelAction::ViewDiff(job_id) => {
                    self.open_job_diff(job_id, ViewMode::JobList)
                }
                DetailPanelAction::Kill(job_id) => self.kill_job(job_id),
                DetailPanelAction::MarkComplete(job_id) => self.mark_job_complete(job_id),
                DetailPanelAction::SetPermissionMode(job_id, mode) => {
                    self.set_job_permission_mode(job_id, mode);
                }
            }
        }
    }

    /// Render the selection popup
    pub(crate) fn render_selection_popup(&mut self, ctx: &egui::Context) {
        use super::selection::{SelectionPopupAction, SelectionPopupState, render_selection_popup};

        let mut state = SelectionPopupState {
            selection: &self.selection,
            popup_input: &mut self.popup_input,
            popup_status: &self.popup_status,
            suggestions: &self.autocomplete.suggestions,
            selected_suggestion: self.autocomplete.selected_suggestion,
            show_suggestions: self.autocomplete.show_suggestions,
            cursor_to_end: &mut self.autocomplete.cursor_to_end,
            voice_state: self.voice_manager.state,
            voice_mode: self.voice_manager.config.mode,
            voice_last_error: self.voice_manager.last_error.as_deref(),
        };

        if let Some(action) = render_selection_popup(ctx, &mut state) {
            match action {
                SelectionPopupAction::InputChanged => {
                    self.update_suggestions();
                }
                SelectionPopupAction::SuggestionClicked(idx) => {
                    self.autocomplete.selected_suggestion = idx;
                    self.apply_suggestion();
                    self.update_suggestions();
                }
                SelectionPopupAction::ToggleRecording => {
                    // Auto-install voice dependencies if not available (async, non-blocking)
                    if !self.voice_manager.is_available() && !self.voice_install_in_progress {
                        self.voice_install_in_progress = true;
                        self.voice_install_status =
                            Some(("Installing voice dependencies...".to_string(), false));

                        let model_name = self.voice_manager.config.whisper_model.clone();
                        // Use async installation to avoid blocking the UI thread
                        let handle = crate::gui::voice::install::install_voice_dependencies_async(
                            &self.work_dir,
                            &model_name,
                        );
                        self.voice_install_handle = Some(handle);

                        self.logs.push(LogEvent::system(
                            "Installing voice dependencies in background...".to_string(),
                        ));
                    } else if !self.voice_install_in_progress {
                        self.voice_manager.toggle_recording();
                    }
                }
            }
        }
    }

    /// Render the batch popup (similar to selection popup but for multiple files)
    pub(crate) fn render_batch_popup(&mut self, ctx: &egui::Context) {
        use super::selection::{BatchPopupState, SelectionPopupAction, render_batch_popup};

        let mut state = BatchPopupState {
            batch_files: &self.batch_files,
            popup_input: &mut self.popup_input,
            popup_status: &self.popup_status,
            suggestions: &self.autocomplete.suggestions,
            selected_suggestion: self.autocomplete.selected_suggestion,
            show_suggestions: self.autocomplete.show_suggestions,
            cursor_to_end: &mut self.autocomplete.cursor_to_end,
        };

        if let Some(action) = render_batch_popup(ctx, &mut state) {
            match action {
                SelectionPopupAction::InputChanged => {
                    self.update_suggestions();
                }
                SelectionPopupAction::SuggestionClicked(idx) => {
                    self.autocomplete.selected_suggestion = idx;
                    self.apply_suggestion();
                    self.update_suggestions();
                }
                SelectionPopupAction::ToggleRecording => {
                    // No voice in batch popup
                }
            }
        }
    }

    /// Render the diff view popup
    pub(crate) fn render_diff_popup(&mut self, ctx: &egui::Context) {
        if super::diff::render_diff_popup(ctx, &self.diff_state) {
            self.view_mode = self.diff_return_view;
            self.diff_state.clear();
        }
    }
}
