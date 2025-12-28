//! Voice dependencies installation section

use eframe::egui::{self, Color32, RichText};

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

pub fn render_dependencies_section(
    ui: &mut egui::Ui,
    voice_install_status: &mut Option<(String, bool)>,
    voice_install_in_progress: &mut bool,
    on_install_dependencies: &mut dyn FnMut(),
) {
    ui.add_space(12.0);
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
                ui.label(RichText::new("-").color(TEXT_DIM));
                ui.label(RichText::new("sox").monospace().color(ACCENT_CYAN));
                ui.label(RichText::new("- audio recording").color(TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("-").color(TEXT_DIM));
                ui.label(RichText::new("whisper-cpp").monospace().color(ACCENT_CYAN));
                ui.label(RichText::new("- speech-to-text transcription").color(TEXT_DIM));
            });

            ui.add_space(12.0);

            render_install_button(ui, voice_install_in_progress, on_install_dependencies);

            if let Some((msg, is_error)) = voice_install_status.as_ref() {
                ui.add_space(8.0);
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).small().color(color));
            }

            // Microphone permission button (macOS only)
            #[cfg(target_os = "macos")]
            render_macos_microphone_settings(ui);
        });
}

fn render_install_button(
    ui: &mut egui::Ui,
    voice_install_in_progress: &mut bool,
    on_install_dependencies: &mut dyn FnMut(),
) {
    ui.horizontal(|ui| {
        let button_text = if *voice_install_in_progress {
            "Installing..."
        } else {
            "Install Voice Dependencies"
        };

        let text_color = if *voice_install_in_progress {
            TEXT_MUTED
        } else {
            TEXT_PRIMARY
        };

        let button = ui.add_enabled(
            !*voice_install_in_progress,
            egui::Button::new(RichText::new(button_text).color(text_color)),
        );

        if button.clicked() && !*voice_install_in_progress {
            (on_install_dependencies)();
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new("(requires Homebrew on macOS)")
                .small()
                .color(TEXT_DIM),
        );
    });
}

#[cfg(target_os = "macos")]
fn render_macos_microphone_settings(ui: &mut egui::Ui) {
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
