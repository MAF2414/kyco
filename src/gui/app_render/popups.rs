//! Popup rendering methods for KycoApp
//!
//! Contains methods for rendering selection and batch popups.

use crate::gui::app::KycoApp;
use crate::gui::selection::{
    BatchPopupState, SelectionPopupAction, SelectionPopupState, render_batch_popup,
    render_selection_popup,
};
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    /// Render the selection popup
    pub(crate) fn render_selection_popup(&mut self, ctx: &egui::Context) {
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
}
