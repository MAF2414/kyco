//! Voice dependencies installation section

use eframe::egui::{self, Color32, RichText};

use crate::gui::settings::state::SettingsState;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::testing::render_voice_test_section;

/// Render voice dependencies installation section
pub fn render_voice_dependencies_section(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Voice Dependencies")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Voice input requires the following tools to be installed:")
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("•").color(TEXT_DIM));
                ui.label(RichText::new("sox").monospace().color(ACCENT_CYAN));
                ui.label(RichText::new("- audio recording").color(TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("•").color(TEXT_DIM));
                ui.label(
                    RichText::new("whisper-cpp")
                        .monospace()
                        .color(ACCENT_CYAN),
                );
                ui.label(RichText::new("- speech-to-text transcription").color(TEXT_DIM));
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let button_text = if *state.voice_install_in_progress {
                    "Installing..."
                } else {
                    "Install Voice Dependencies"
                };

                let button = ui.add_enabled(
                    !*state.voice_install_in_progress,
                    egui::Button::new(RichText::new(button_text).color(
                        if *state.voice_install_in_progress {
                            TEXT_MUTED
                        } else {
                            TEXT_PRIMARY
                        },
                    )),
                );

                if button.clicked() && !*state.voice_install_in_progress {
                    install_voice_dependencies(state);
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new("(requires Homebrew on macOS)")
                        .small()
                        .color(TEXT_DIM),
                );
            });

            if let Some((msg, is_error)) = &state.voice_install_status {
                ui.add_space(8.0);
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg).small().color(color));
            }

            // Microphone permission button (macOS only)
            #[cfg(target_os = "macos")]
            {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Microphone access must be granted in System Settings.")
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.add_space(4.0);
                if ui
                    .button(RichText::new("Open Microphone Settings").color(ACCENT_CYAN))
                    .clicked()
                {
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                        .spawn();
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            render_voice_test_section(ui, state);
        });
}

/// Install voice dependencies (sox, whisper-cpp) - async, non-blocking
fn install_voice_dependencies(state: &mut SettingsState<'_>) {
    *state.voice_install_in_progress = true;
    *state.voice_install_status = Some(("Installing voice dependencies...".to_string(), false));

    // Use the selected model from settings (defaults to "base")
    let model_name = if state.voice_settings_model.is_empty() {
        "base"
    } else {
        state.voice_settings_model.as_str()
    };

    // Use async installation to avoid blocking the UI thread
    let handle =
        crate::gui::voice::install::install_voice_dependencies_async(state.work_dir, model_name);
    *state.voice_install_handle = Some(handle);
}
