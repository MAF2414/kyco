//! Theme and UI styling for KycoApp
//!
//! Contains theme application and init banner rendering.

use super::app::KycoApp;
use super::theme::{ACCENT_YELLOW, BG_HIGHLIGHT, BG_PRIMARY, BG_SECONDARY, TEXT_PRIMARY};
use crate::LogEvent;
use crate::config::Config;
use eframe::egui::{self, Stroke};

impl KycoApp {
    /// Apply the dark theme to the egui context.
    pub(crate) fn apply_theme(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = BG_PRIMARY;
        style.visuals.window_fill = BG_PRIMARY;
        style.visuals.extreme_bg_color = BG_SECONDARY;
        style.visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
        style.visuals.widgets.inactive.bg_fill = BG_SECONDARY;
        style.visuals.widgets.hovered.bg_fill = BG_HIGHLIGHT;
        style.visuals.widgets.active.bg_fill = BG_HIGHLIGHT;
        style.visuals.selection.bg_fill = BG_HIGHLIGHT;
        style.visuals.selection.stroke = Stroke::new(1.0, TEXT_PRIMARY);
        ctx.set_style(style);
    }

    /// Render the init banner if no config exists.
    /// Returns true if the banner was shown.
    pub(crate) fn render_init_banner(&mut self, ctx: &egui::Context) -> bool {
        if self.config_exists {
            return false;
        }

        egui::TopBottomPanel::top("init_banner")
            .frame(egui::Frame::NONE.fill(ACCENT_YELLOW).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⚠ No configuration found.")
                            .color(BG_PRIMARY)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    if ui
                        .button(
                            egui::RichText::new("Initialize Project")
                                .color(BG_PRIMARY)
                                .strong(),
                        )
                        .clicked()
                    {
                        // Create global config at ~/.kyco/config.toml
                        let config_dir = Config::global_config_dir();
                        let config_path = Config::global_config_path();
                        if let Err(e) = std::fs::create_dir_all(&config_dir) {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to create config directory: {}",
                                e
                            )));
                        } else if let Err(e) = Config::with_defaults().save_to_file(&config_path) {
                            self.logs
                                .push(LogEvent::error(format!("Failed to write config: {}", e)));
                        } else {
                            self.config_exists = true;
                            self.logs.push(LogEvent::system(format!(
                                "Created {}",
                                config_path.display()
                            )));
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Working directory: {}",
                            self.work_dir.display()
                        ))
                        .color(BG_PRIMARY)
                        .small(),
                    );
                });
            });

        true
    }
}
