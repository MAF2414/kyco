//! Voice hotkey handling for input processing

use super::InputAction;
use crate::gui::app::KycoApp;
use crate::gui::hotkey::check_egui_hotkey;
use crate::gui::voice::VoiceState;
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    pub(super) fn handle_voice_hotkey(&mut self, i: &egui::InputState) -> Option<InputAction> {
        let voice_hotkey_pressed = check_egui_hotkey(i, &self.voice_settings_popup_hotkey);

        if !voice_hotkey_pressed {
            return None;
        }

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
            return Some(InputAction::InstallVoiceDeps);
        }

        if !self.voice_install_in_progress {
            if self.voice_manager.state == VoiceState::Idle
                || self.voice_manager.state == VoiceState::Error
            {
                // Start recording
                self.voice_manager.start_recording();
                return Some(InputAction::StartVoiceRecording);
            } else if self.voice_manager.state == VoiceState::Recording {
                // Stop recording (but don't execute - user can press Enter for that)
                self.voice_manager.stop_recording();
                return Some(InputAction::StopVoiceRecording);
            }
        }

        None
    }
}
