//! VAD (Voice Activity Detection) settings UI

use eframe::egui::{self, Color32, RichText};

use crate::gui::theme::{TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

pub fn render_vad_settings(
    ui: &mut egui::Ui,
    vad_enabled: &mut bool,
    vad_speech_threshold: &mut String,
    vad_silence_duration_ms: &mut String,
) {
    ui.label(
        RichText::new("VAD Settings (Voice Activity Detection)")
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
                RichText::new(
                    "VAD detects when you start/stop speaking for efficient continuous listening.",
                )
                .small()
                .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.checkbox(vad_enabled, "");
                ui.label(RichText::new("Enable VAD for continuous mode").color(TEXT_PRIMARY));
            });

            if *vad_enabled {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Speech threshold:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(vad_speech_threshold)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(60.0),
                    );
                    ui.label(RichText::new("(0.0-1.0)").small().color(TEXT_DIM));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Silence to stop:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(vad_silence_duration_ms)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(60.0),
                    );
                    ui.label(RichText::new("ms").small().color(TEXT_DIM));
                });
            }
        });
}
