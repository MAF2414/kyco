//! Keyboard input handling for KycoApp
//!
//! Contains keyboard shortcut processing extracted from the main update loop.

mod voice_hotkey;

use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use crate::gui::voice::VoiceInputMode;
use eframe::egui::{self, Key};

/// Action returned from keyboard input processing
#[allow(dead_code)] // Some variants reserved for future voice features
pub(crate) enum InputAction {
    None,
    /// Execute popup task with optional force_worktree flag
    ExecutePopup { force_worktree: bool },
    /// Execute batch task with optional force_worktree flag
    ExecuteBatch { force_worktree: bool },
    /// Voice recording should start
    StartVoiceRecording,
    /// Voice recording should stop
    StopVoiceRecording,
    /// Voice recording should stop and execute after transcription
    StopVoiceAndExecute,
    /// Install voice dependencies
    InstallVoiceDeps,
}

impl KycoApp {
    /// Process keyboard shortcuts based on current view mode.
    /// Returns an action to be executed after input processing.
    pub(crate) fn process_keyboard_input(&mut self, i: &egui::InputState) -> InputAction {
        let mut action = InputAction::None;

        match self.view_mode {
            ViewMode::SelectionPopup => {
                action = self.handle_selection_popup_input(i);
            }
            ViewMode::BatchPopup => {
                action = self.handle_batch_popup_input(i);
            }
            ViewMode::DiffView => {
                self.handle_diff_view_input(i);
            }
            ViewMode::ApplyConfirmPopup => {
                self.handle_apply_confirm_input(i);
            }
            ViewMode::ComparisonPopup => {
                self.handle_comparison_popup_input(i);
            }
            ViewMode::JobList => {
                self.handle_job_list_input(i);
            }
            ViewMode::Settings => {
                self.handle_settings_input(i);
            }
            ViewMode::Skills => {
                self.handle_skills_input(i);
            }
            ViewMode::Agents => {
                self.handle_agents_input(i);
            }
            ViewMode::Chains => {
                self.handle_chains_input(i);
            }
            ViewMode::Files => {
                self.handle_files_input(i);
            }
            ViewMode::Stats => {
                self.handle_stats_input(i);
            }
            ViewMode::Achievements => {
                self.handle_achievements_input(i);
            }
            ViewMode::Kanban => {
                // Kanban board - ESC returns to job list
                if i.key_pressed(Key::Escape) {
                    self.view_mode = ViewMode::JobList;
                }
            }
        }

        // Global shortcut for auto_run toggle (Shift+A)
        if i.modifiers.shift && i.key_pressed(Key::A) {
            self.auto_run = !self.auto_run;
        }

        // Voice hotkey handling (configurable, default: Cmd+D / Ctrl+D)
        if self.view_mode == ViewMode::SelectionPopup {
            if let Some(voice_action) = self.handle_voice_hotkey(i) {
                action = voice_action;
            }
        }

        // Global shortcut to toggle continuous listening (Shift+L)
        if i.modifiers.shift && i.key_pressed(Key::L) {
            if self.voice_manager.config.mode == VoiceInputMode::Continuous {
                self.voice_manager.toggle_listening();
            }
        }

        action
    }

    fn handle_selection_popup_input(&mut self, i: &egui::InputState) -> InputAction {
        if i.key_pressed(Key::Escape) {
            // Cancel recording if active, otherwise close popup
            if self.voice_manager.state.is_recording() {
                self.voice_manager.cancel();
                self.voice_pending_execute = false;
            } else {
                self.view_mode = ViewMode::JobList;
            }
        }

        if i.key_pressed(Key::Tab)
            && self.autocomplete.show_suggestions
            && !self.autocomplete.suggestions.is_empty()
        {
            self.apply_suggestion();
            self.update_suggestions();
        }

        if i.key_pressed(Key::ArrowDown) && self.autocomplete.show_suggestions {
            self.autocomplete.select_next();
        }

        if i.key_pressed(Key::ArrowUp) && self.autocomplete.show_suggestions {
            self.autocomplete.select_previous();
        }

        if i.key_pressed(Key::Enter) {
            if self.voice_manager.state.is_recording() {
                // Stop recording and execute after transcription
                self.voice_pending_execute = true;
                self.voice_manager.stop_recording();
            } else if !self.voice_manager.state.is_busy() {
                // Normal execution (not recording/transcribing)
                let force_worktree = i.modifiers.shift;
                return InputAction::ExecutePopup { force_worktree };
            }
            // If transcribing, do nothing - wait for completion
        }

        InputAction::None
    }

    fn handle_batch_popup_input(&mut self, i: &egui::InputState) -> InputAction {
        if i.key_pressed(Key::Escape) {
            self.batch_files.clear();
            self.view_mode = ViewMode::JobList;
        }

        if i.key_pressed(Key::Tab)
            && self.autocomplete.show_suggestions
            && !self.autocomplete.suggestions.is_empty()
        {
            self.apply_suggestion();
            self.update_suggestions();
        }

        if i.key_pressed(Key::ArrowDown) && self.autocomplete.show_suggestions {
            self.autocomplete.select_next();
        }

        if i.key_pressed(Key::ArrowUp) && self.autocomplete.show_suggestions {
            self.autocomplete.select_previous();
        }

        if i.key_pressed(Key::Enter) {
            let force_worktree = i.modifiers.shift;
            return InputAction::ExecuteBatch { force_worktree };
        }

        InputAction::None
    }

    fn handle_diff_view_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
            self.view_mode = self.diff_return_view;
            self.diff_state.clear();
        }
    }

    fn handle_apply_confirm_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
            if self.apply_confirm_rx.is_none() {
                self.apply_confirm_target = None;
                self.apply_confirm_error = None;
                self.view_mode = self.apply_confirm_return_view;
            }
        }
        if i.key_pressed(Key::Enter) && self.apply_confirm_rx.is_none() {
            self.start_apply_confirm_merge();
        }
    }

    fn handle_comparison_popup_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
            self.comparison_state.close();
            self.view_mode = ViewMode::JobList;
        }
    }

    fn handle_job_list_input(&mut self, i: &egui::InputState) {
        // Navigate jobs with j/k or arrows
        if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
            // Select next job
            if let Some(current_id) = self.selected_job_id {
                if let Some(idx) = self.cached_jobs.iter().position(|j| j.id == current_id) {
                    if idx + 1 < self.cached_jobs.len() {
                        self.selected_job_id = Some(self.cached_jobs[idx + 1].id);
                    }
                }
            } else if !self.cached_jobs.is_empty() {
                self.selected_job_id = Some(self.cached_jobs[0].id);
            }
        }

        if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
            // Select previous job
            if let Some(current_id) = self.selected_job_id {
                if let Some(idx) = self.cached_jobs.iter().position(|j| j.id == current_id) {
                    if idx > 0 {
                        self.selected_job_id = Some(self.cached_jobs[idx - 1].id);
                    }
                }
            }
        }
    }

    fn handle_settings_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            self.view_mode = ViewMode::JobList;
        }
    }

    fn handle_skills_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            if self.selected_mode.is_some() {
                self.selected_mode = None;
                self.mode_edit_status = None;
            } else {
                self.view_mode = ViewMode::JobList;
            }
        }
    }

    fn handle_agents_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            if self.selected_agent.is_some() {
                self.selected_agent = None;
                self.agent_edit_status = None;
            } else {
                self.view_mode = ViewMode::JobList;
            }
        }
    }

    fn handle_chains_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            if self.selected_chain.is_some() {
                self.selected_chain = None;
                self.chain_edit_status = None;
            } else {
                self.view_mode = ViewMode::JobList;
            }
        }
    }

    fn handle_files_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            // Clear search and return to job list
            self.file_search.search_query.clear();
            self.file_search.search_results.clear();
            self.file_search.selected_files.clear();
            self.view_mode = ViewMode::JobList;
        }
    }

    fn handle_stats_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            self.view_mode = ViewMode::JobList;
        }
    }

    fn handle_achievements_input(&mut self, i: &egui::InputState) {
        if i.key_pressed(Key::Escape) {
            self.view_mode = ViewMode::Stats;
        }
    }
}
