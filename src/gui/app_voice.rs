//! Voice input methods for KycoApp
//!
//! This module contains voice recording, transcription, and overlay rendering
//! extracted from app.rs for better organization.

use super::hotkey::parse_hotkey_string;
use super::voice::{VoiceConfig, VoiceInputMode, VoiceState, copy_and_paste};
use crate::LogEvent;
use eframe::egui::{self, Color32};
use global_hotkey::{
    GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};

use super::app::KycoApp;

impl KycoApp {
    /// Handle global voice hotkey press (Cmd+Shift+V / Ctrl+Shift+V)
    ///
    /// This toggles voice recording from any application:
    /// - First press: Start recording, show overlay
    /// - Second press: Stop recording, transcribe, auto-paste to focused app
    pub(crate) fn handle_global_voice_hotkey(&mut self) {
        // Auto-install voice dependencies if not available (async, non-blocking)
        if !self.voice_manager.is_available() && !self.voice_install_in_progress {
            self.voice_install_in_progress = true;
            self.voice_install_status =
                Some(("Installing voice dependencies...".to_string(), false));

            let model_name = self.voice_manager.config.whisper_model.clone();
            // Use async installation to avoid blocking the UI thread
            let handle = super::voice::install::install_voice_dependencies_async(
                &self.work_dir,
                &model_name,
            );
            self.voice_install_handle = Some(handle);

            self.logs.push(LogEvent::system(
                "Installing voice dependencies in background...".to_string(),
            ));
            return; // Installation started, will be polled in update loop
        }

        if self.voice_install_in_progress {
            return; // Still installing
        }

        match self.voice_manager.state {
            VoiceState::Idle | VoiceState::Error => {
                // Start recording
                self.voice_manager.start_recording();
                self.global_voice_recording = true;
                self.show_voice_overlay = true;
                self.logs.push(LogEvent::system(
                    "🎤 Global voice recording started (Cmd+Shift+V to stop)".to_string(),
                ));
            }
            VoiceState::Recording => {
                // Stop recording and transcribe
                self.voice_manager.stop_recording();
                // Note: global_voice_recording stays true until transcription completes
                self.logs.push(LogEvent::system(
                    "⏳ Stopping recording, transcribing...".to_string(),
                ));
            }
            VoiceState::Transcribing => {
                // Already transcribing, ignore
                self.logs.push(LogEvent::system(
                    "⏳ Already transcribing, please wait...".to_string(),
                ));
            }
            _ => {}
        }
    }

    /// Handle completed transcription for global voice input
    pub(crate) fn handle_global_voice_transcription(&mut self, text: &str) {
        self.global_voice_recording = false;
        self.show_voice_overlay = false;

        if self.global_voice_auto_paste {
            // Copy to clipboard and auto-paste
            match copy_and_paste(text, true) {
                Ok(()) => {
                    self.logs.push(LogEvent::system(format!(
                        "✓ Voice transcribed and pasted: \"{}\"",
                        if text.chars().count() > 50 {
                            let end = text
                                .char_indices()
                                .nth(50)
                                .map(|(i, _)| i)
                                .unwrap_or(text.len());
                            format!("{}...", &text[..end])
                        } else {
                            text.to_string()
                        }
                    )));
                }
                Err(e) => {
                    // Paste failed but text is in clipboard
                    self.logs.push(LogEvent::system(format!(
                        "Voice transcribed (use Cmd+V to paste): {}",
                        e
                    )));
                }
            }
        } else {
            // Just copy to clipboard, no auto-paste
            if let Err(e) = copy_and_paste(text, false) {
                self.logs.push(LogEvent::error(format!(
                    "Failed to copy to clipboard: {}",
                    e
                )));
            } else {
                self.logs.push(LogEvent::system(format!(
                    "✓ Voice transcribed and copied: \"{}\"",
                    if text.chars().count() > 50 {
                        let end = text
                            .char_indices()
                            .nth(50)
                            .map(|(i, _)| i)
                            .unwrap_or(text.len());
                        format!("{}...", &text[..end])
                    } else {
                        text.to_string()
                    }
                )));
            }
        }
    }

    /// Render a small voice recording overlay in the corner of the screen
    pub(crate) fn render_voice_overlay(&self, ctx: &egui::Context) {
        let state_text = match self.voice_manager.state {
            VoiceState::Recording => "🎤 Recording...",
            VoiceState::Transcribing => "⏳ Transcribing...",
            _ => "🎤 Voice Input",
        };

        egui::Window::new("voice_overlay")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-20.0, 20.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(if self.voice_manager.state == VoiceState::Recording {
                        Color32::from_rgb(200, 60, 60) // Red when recording
                    } else {
                        Color32::from_rgb(60, 60, 80) // Dark when transcribing
                    })
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(state_text)
                            .color(Color32::WHITE)
                            .size(14.0)
                            .strong(),
                    );
                });
                if self.voice_manager.state == VoiceState::Recording {
                    ui.label(
                        egui::RichText::new("Press Cmd+Shift+V to stop")
                            .color(Color32::from_gray(200))
                            .size(11.0),
                    );
                }
            });
    }

    /// Initialize the global hotkey manager and register voice hotkey
    pub(crate) fn init_global_hotkey_manager(hotkey_str: &str) -> Option<GlobalHotKeyManager> {
        match GlobalHotKeyManager::new() {
            Ok(manager) => {
                // Parse the configured hotkey string
                let (modifiers, code) = match parse_hotkey_string(hotkey_str) {
                    Some((m, c)) => (m, c),
                    None => {
                        tracing::warn!(
                            "Invalid hotkey string '{}', using default Cmd+Shift+V",
                            hotkey_str
                        );
                        // Fallback to default
                        #[cfg(target_os = "macos")]
                        let default_mods = Modifiers::SUPER | Modifiers::SHIFT;
                        #[cfg(not(target_os = "macos"))]
                        let default_mods = Modifiers::CONTROL | Modifiers::SHIFT;
                        (default_mods, Code::KeyV)
                    }
                };

                let hotkey = HotKey::new(Some(modifiers), code);

                if let Err(e) = manager.register(hotkey) {
                    tracing::warn!("Failed to register global voice hotkey: {}", e);
                    return Some(manager);
                }

                tracing::info!("Global voice hotkey registered: {}", hotkey_str);
                Some(manager)
            }
            Err(e) => {
                tracing::warn!("Failed to create global hotkey manager: {}", e);
                None
            }
        }
    }

    /// Apply voice config from settings to the VoiceManager
    pub(crate) fn apply_voice_config(&mut self) {
        let Ok(config) = self.config.read() else {
            return; // Skip if lock poisoned
        };
        let voice_settings = &config.settings.gui.voice;
        let action_registry = super::voice::VoiceActionRegistry::from_config(
            &config.mode,
            &config.chain,
            &config.agent,
        );

        let new_config = VoiceConfig {
            mode: match voice_settings.mode.as_str() {
                "manual" => VoiceInputMode::Manual,
                "hotkey_hold" => VoiceInputMode::HotkeyHold,
                "continuous" => VoiceInputMode::Continuous,
                _ => VoiceInputMode::Disabled,
            },
            keywords: voice_settings.keywords.clone(),
            action_registry,
            whisper_model: voice_settings.whisper_model.clone(),
            language: voice_settings.language.clone(),
            silence_threshold: voice_settings.silence_threshold,
            silence_duration: voice_settings.silence_duration,
            max_duration: voice_settings.max_duration,
            vad_config: self.voice_manager.config.vad_config.clone(),
            use_vad: self.voice_manager.config.use_vad,
        };

        self.voice_manager.update_config(new_config);
        self.logs.push(crate::LogEvent::system(
            "Voice settings applied".to_string(),
        ));
    }
}
