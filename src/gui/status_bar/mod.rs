//! Status bar module for the GUI
//!
//! Renders the bottom status bar with auto-run toggle,
//! settings button, modes button, agents button, workspace selector, and update notifications.

use eframe::egui::{self, RichText};
use std::sync::{Arc, Mutex};

use crate::gui::animations::animated_button;

/// Compile-time version string to avoid runtime allocation
const VERSION_TEXT: &str = concat!("kyco v", env!("CARGO_PKG_VERSION"));

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY, ViewMode,
};
use crate::gui::update::{open_url, UpdateInfo};

/// GitHub Sponsors URL
const SPONSOR_URL: &str = "https://github.com/sponsors/MAF2414";
use crate::workspace::{WorkspaceId, WorkspaceRegistry};

/// Install status for update button
#[derive(Debug, Clone, Default)]
pub enum InstallStatus {
    #[default]
    Ready,
    /// User clicked install - app should start installation
    InstallRequested,
    Installing,
    Success(String),
    Error(String),
}

/// Status bar state that can be modified by the status bar UI
pub struct StatusBarState<'a> {
    pub auto_run: &'a mut bool,
    pub view_mode: &'a mut ViewMode,
    pub selected_mode: &'a mut Option<String>,
    pub mode_edit_status: &'a mut Option<(String, bool)>,
    pub selected_agent: &'a mut Option<String>,
    pub agent_edit_status: &'a mut Option<(String, bool)>,
    pub selected_chain: &'a mut Option<String>,
    pub chain_edit_status: &'a mut Option<(String, bool)>,
    /// Update info if available
    pub update_info: Option<&'a UpdateInfo>,
    /// Install status
    pub install_status: &'a mut InstallStatus,
    /// Workspace registry for multi-workspace support
    pub workspace_registry: Option<&'a Arc<Mutex<WorkspaceRegistry>>>,
    /// Currently active workspace ID
    pub active_workspace_id: &'a mut Option<WorkspaceId>,
    /// User requested to launch an external orchestrator session
    pub orchestrator_requested: &'a mut bool,
}

/// Render the bottom status bar
pub fn render_status_bar(ctx: &egui::Context, state: &mut StatusBarState<'_>) {
    egui::TopBottomPanel::bottom("status_bar")
        .frame(egui::Frame::NONE.fill(BG_SECONDARY).inner_margin(4.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
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

                // Workspace selector (if multiple workspaces exist)
                if let Some(registry_arc) = state.workspace_registry {
                    if let Ok(registry) = registry_arc.lock() {
                        let workspaces = registry.list();
                        if workspaces.len() > 1 {
                            let current_name: &str = state
                                .active_workspace_id
                                .and_then(|id| registry.get(id))
                                .map(|ws| ws.name.as_str())
                                .unwrap_or("No workspace");

                            ui.label(RichText::new("Workspace:").small().color(TEXT_DIM));

                            egui::ComboBox::from_id_salt("workspace_selector")
                                .selected_text(
                                    RichText::new(current_name).small().color(TEXT_PRIMARY),
                                )
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for ws in &workspaces {
                                        let is_selected = state
                                            .active_workspace_id
                                            .map_or(false, |id| id == ws.id);
                                        let text = RichText::new(ws.name.as_str()).small();
                                        if ui.selectable_label(is_selected, text).clicked() {
                                            *state.active_workspace_id = Some(ws.id);
                                        }
                                    }
                                });
                        } else if !workspaces.is_empty() {
                            // Single workspace - just show the name with folder icon prefix
                            ui.label(RichText::new("📁 ").small().color(TEXT_DIM));
                            ui.label(
                                RichText::new(workspaces[0].name.as_str())
                                    .small()
                                    .color(TEXT_DIM),
                            );
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(VERSION_TEXT).small().color(TEXT_MUTED));

                    // Sponsor heart button
                    ui.add_space(8.0);
                    if ui
                        .label(RichText::new("♥").small().color(ACCENT_RED))
                        .on_hover_text("Support KYCo on GitHub Sponsors")
                        .clicked()
                    {
                        open_url(SPONSOR_URL);
                    }

                    // Show update notification/install status
                    match state.install_status {
                        InstallStatus::Ready => {
                            if let Some(update_info) = state.update_info {
                                ui.add_space(8.0);
                                let update_text = format!("⬆ Install v{}", update_info.version);
                                if ui
                                    .button(RichText::new(&update_text).small().color(ACCENT_GREEN))
                                    .on_hover_text("Click to download and install update")
                                    .clicked()
                                {
                                    *state.install_status = InstallStatus::InstallRequested;
                                }
                            }
                        }
                        InstallStatus::InstallRequested | InstallStatus::Installing => {
                            ui.add_space(8.0);
                            ui.spinner();
                            ui.label(RichText::new("Installing...").small().color(ACCENT_YELLOW));
                        }
                        InstallStatus::Success(msg) => {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("✓ {}", msg))
                                    .small()
                                    .color(ACCENT_GREEN),
                            );
                        }
                        InstallStatus::Error(err) => {
                            ui.add_space(8.0);
                            if ui
                                .label(
                                    RichText::new(format!("✗ {}", err))
                                        .small()
                                        .color(ACCENT_RED),
                                )
                                .on_hover_text("Update failed - click to retry")
                                .clicked()
                            {
                                *state.install_status = InstallStatus::InstallRequested;
                            }
                        }
                    }

                    ui.add_space(16.0);
                    if animated_button(ui, "Settings", ACCENT_CYAN, "statusbar_settings").clicked()
                    {
                        *state.view_mode = ViewMode::Settings;
                    }
                    ui.add_space(8.0);
                    if animated_button(ui, "Modes", ACCENT_PURPLE, "statusbar_modes").clicked() {
                        *state.view_mode = ViewMode::Modes;
                        *state.selected_mode = None;
                        *state.mode_edit_status = None;
                    }
                    ui.add_space(8.0);
                    if animated_button(ui, "Agents", TEXT_PRIMARY, "statusbar_agents").clicked() {
                        *state.view_mode = ViewMode::Agents;
                        *state.selected_agent = None;
                        *state.agent_edit_status = None;
                    }
                    ui.add_space(8.0);
                    if animated_button(ui, "Chains", ACCENT_YELLOW, "statusbar_chains").clicked() {
                        *state.view_mode = ViewMode::Chains;
                        *state.selected_chain = None;
                        *state.chain_edit_status = None;
                    }
                    ui.add_space(8.0);
                    if animated_button(ui, "Orchestrator", ACCENT_GREEN, "statusbar_orchestrator")
                        .clicked()
                    {
                        *state.orchestrator_requested = true;
                    }
                });
            });
        });
}
