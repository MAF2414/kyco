//! Status bar module for the GUI
//!
//! Renders the bottom status bar with auto-run/auto-scan toggles,
//! settings button, modes button, agents button, and update notifications.

use eframe::egui::{self, RichText};

use crate::gui::app::{
    ViewMode, ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_SECONDARY, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::gui::update::UpdateInfo;

/// Status bar state that can be modified by the status bar UI
pub struct StatusBarState<'a> {
    pub auto_run: &'a mut bool,
    pub auto_scan: &'a mut bool,
    pub view_mode: &'a mut ViewMode,
    pub selected_mode: &'a mut Option<String>,
    pub mode_edit_status: &'a mut Option<(String, bool)>,
    pub selected_agent: &'a mut Option<String>,
    pub agent_edit_status: &'a mut Option<(String, bool)>,
    pub selected_chain: &'a mut Option<String>,
    pub chain_edit_status: &'a mut Option<(String, bool)>,
    /// Update info if available
    pub update_info: Option<&'a UpdateInfo>,
}

/// Render the bottom status bar
pub fn render_status_bar(ctx: &egui::Context, state: &mut StatusBarState<'_>) {
    egui::TopBottomPanel::bottom("status_bar")
        .frame(egui::Frame::NONE.fill(BG_SECONDARY).inner_margin(4.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Auto-run toggle
                let auto_run_text = if *state.auto_run {
                    "[A]utoRun: ON"
                } else {
                    "[A]utoRun: off"
                };
                let auto_run_color = if *state.auto_run {
                    ACCENT_GREEN
                } else {
                    TEXT_MUTED
                };
                if ui
                    .label(
                        RichText::new(auto_run_text)
                            .small()
                            .monospace()
                            .color(auto_run_color),
                    )
                    .clicked()
                {
                    *state.auto_run = !*state.auto_run;
                }

                ui.add_space(16.0);

                // Auto-scan toggle
                let auto_scan_text = if *state.auto_scan {
                    "[S]can: ON"
                } else {
                    "[S]can: off"
                };
                let auto_scan_color = if *state.auto_scan {
                    ACCENT_GREEN
                } else {
                    TEXT_MUTED
                };
                if ui
                    .label(
                        RichText::new(auto_scan_text)
                            .small()
                            .monospace()
                            .color(auto_scan_color),
                    )
                    .clicked()
                {
                    *state.auto_scan = !*state.auto_scan;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show current version
                    let version_text = format!("kyco v{}", env!("CARGO_PKG_VERSION"));
                    ui.label(RichText::new(&version_text).small().color(TEXT_MUTED));

                    // Show update notification if available
                    if let Some(update_info) = state.update_info {
                        ui.add_space(8.0);
                        let update_text = format!("⬆ v{} available", update_info.version);
                        if ui
                            .button(RichText::new(&update_text).small().color(ACCENT_GREEN))
                            .on_hover_text("Click to open download page")
                            .clicked()
                        {
                            crate::gui::update::open_url(&update_info.release_url);
                        }
                    }

                    ui.add_space(16.0);
                    if ui
                        .button(RichText::new("⚙ Settings").small().color(ACCENT_CYAN))
                        .clicked()
                    {
                        *state.view_mode = ViewMode::Settings;
                    }
                    ui.add_space(8.0);
                    if ui
                        .button(RichText::new("📋 Modes").small().color(ACCENT_PURPLE))
                        .clicked()
                    {
                        *state.view_mode = ViewMode::Modes;
                        *state.selected_mode = None;
                        *state.mode_edit_status = None;
                    }
                    ui.add_space(8.0);
                    if ui
                        .button(RichText::new("🤖 Agents").small().color(TEXT_PRIMARY))
                        .clicked()
                    {
                        *state.view_mode = ViewMode::Agents;
                        *state.selected_agent = None;
                        *state.agent_edit_status = None;
                    }
                    ui.add_space(8.0);
                    if ui
                        .button(RichText::new("🔗 Chains").small().color(ACCENT_YELLOW))
                        .clicked()
                    {
                        *state.view_mode = ViewMode::Chains;
                        *state.selected_chain = None;
                        *state.chain_edit_status = None;
                    }
                });
            });
        });
}
