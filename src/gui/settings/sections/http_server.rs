//! HTTP server settings section

use eframe::egui::{self, RichText};

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render HTTP Server info section
pub fn render_settings_http_server(ui: &mut egui::Ui) {
    ui.label(RichText::new("HTTP Server").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Status:").color(TEXT_MUTED));
        ui.label(RichText::new("● Running").color(ACCENT_GREEN));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Address:").color(TEXT_MUTED));
        ui.label(
            RichText::new("http://127.0.0.1:9876")
                .monospace()
                .color(ACCENT_CYAN),
        );
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new("Extensions send selections to this endpoint.")
            .small()
            .color(TEXT_DIM),
    );
}
