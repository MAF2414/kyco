//! Voice event handling for KycoApp update loop

use crate::LogEvent;
use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use eframe::egui;

impl KycoApp {
    /// Handle voice events from VoiceManager
    pub(crate) fn handle_voice_events(&mut self, ctx: &egui::Context) {
        for event in self.voice_manager.poll_events() {
            match event {
                super::super::voice::VoiceEvent::TranscriptionComplete { text, from_manual } => {
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
                            let (detected_mode, prompt) = super::super::voice::parse_voice_input(
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
                super::super::voice::VoiceEvent::WakewordMatched {
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
                super::super::voice::VoiceEvent::KeywordDetected { keyword, full_text } => {
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
                super::super::voice::VoiceEvent::Error { message } => {
                    self.logs
                        .push(LogEvent::error(format!("Voice error: {}", message)));
                    self.voice_pending_execute = false; // Cancel pending execution on error
                    // Reset global voice state on error
                    if self.global_voice_recording {
                        self.global_voice_recording = false;
                        self.show_voice_overlay = false;
                    }
                }
                super::super::voice::VoiceEvent::RecordingStarted => {
                    self.logs
                        .push(LogEvent::system("Voice recording started".to_string()));
                }
                super::super::voice::VoiceEvent::RecordingStopped { duration_secs } => {
                    self.logs.push(LogEvent::system(format!(
                        "Voice recording stopped ({:.1}s)",
                        duration_secs
                    )));
                }
                super::super::voice::VoiceEvent::VadSpeechStarted => {
                    self.logs
                        .push(LogEvent::system("VAD: Speech detected".to_string()));
                }
                super::super::voice::VoiceEvent::VadSpeechEnded => {
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
                use super::super::voice::install::InstallProgress;
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
}
