//! Dashboard header with filters and controls

use eframe::egui::{self, RichText};

use crate::gui::animations::animated_button;
use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_RED, TEXT_DIM};
use crate::stats::TimeRange;

impl KycoApp {
    pub(super) fn render_dashboard_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("DASHBOARD").monospace().size(18.0).color(crate::gui::theme::TEXT_PRIMARY));
            ui.add_space(16.0);

            // Time range selector
            ui.label(RichText::new("Range:").small().color(TEXT_DIM));
            egui::ComboBox::from_id_salt("stats_time_range")
                .selected_text(self.stats_time_range.label())
                .show_ui(ui, |ui| {
                    for range in [TimeRange::Last7Days, TimeRange::Last30Days, TimeRange::Last90Days, TimeRange::AllTime] {
                        if ui.selectable_label(self.stats_time_range == range, range.label()).clicked() {
                            self.stats_time_range = range;
                            self.refresh_dashboard();
                        }
                    }
                });

            ui.add_space(12.0);

            // Agent filter
            ui.label(RichText::new("Agent:").small().color(TEXT_DIM));
            let agent_label = self.stats_filter_agent.as_deref().unwrap_or("All");
            let available_agents = self.dashboard_summary.available_agents.clone();
            let mut agent_changed = false;
            let mut new_agent: Option<String> = self.stats_filter_agent.clone();
            egui::ComboBox::from_id_salt("stats_filter_agent")
                .selected_text(agent_label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(new_agent.is_none(), "All").clicked() {
                        new_agent = None;
                        agent_changed = true;
                    }
                    for agent in &available_agents {
                        let selected = new_agent.as_ref() == Some(agent);
                        if ui.selectable_label(selected, agent).clicked() {
                            new_agent = Some(agent.clone());
                            agent_changed = true;
                        }
                    }
                });
            if agent_changed {
                self.stats_filter_agent = new_agent;
            }

            ui.add_space(12.0);

            // Mode filter
            ui.label(RichText::new("Mode:").small().color(TEXT_DIM));
            let mode_label = self.stats_filter_mode.as_deref().unwrap_or("All");
            let available_modes = self.dashboard_summary.available_modes.clone();
            let mut mode_changed = false;
            let mut new_mode: Option<String> = self.stats_filter_mode.clone();
            egui::ComboBox::from_id_salt("stats_filter_mode")
                .selected_text(mode_label)
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(new_mode.is_none(), "All").clicked() {
                        new_mode = None;
                        mode_changed = true;
                    }
                    for mode in &available_modes {
                        let selected = new_mode.as_ref() == Some(mode);
                        if ui.selectable_label(selected, mode).clicked() {
                            new_mode = Some(mode.clone());
                            mode_changed = true;
                        }
                    }
                });
            if mode_changed {
                self.stats_filter_mode = new_mode;
            }

            ui.add_space(12.0);

            // Workspace filter
            ui.label(RichText::new("Workspace:").small().color(TEXT_DIM));
            let workspace_label = self.stats_filter_workspace
                .as_ref()
                .map(|w| {
                    // Show just the last path component
                    w.rsplit('/').next().unwrap_or(w)
                })
                .unwrap_or("All");
            let available_workspaces = self.dashboard_summary.available_workspaces.clone();
            let mut workspace_changed = false;
            let mut new_workspace: Option<String> = self.stats_filter_workspace.clone();
            egui::ComboBox::from_id_salt("stats_filter_workspace")
                .selected_text(workspace_label)
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(new_workspace.is_none(), "All").clicked() {
                        new_workspace = None;
                        workspace_changed = true;
                    }
                    for ws in &available_workspaces {
                        let selected = new_workspace.as_ref() == Some(ws);
                        // Show short name in dropdown
                        let display = ws.rsplit('/').next().unwrap_or(ws);
                        if ui.selectable_label(selected, display).clicked() {
                            new_workspace = Some(ws.clone());
                            workspace_changed = true;
                        }
                    }
                });
            if workspace_changed {
                self.stats_filter_workspace = new_workspace;
            }

            // Trigger refresh if filters changed
            if agent_changed || mode_changed || workspace_changed {
                self.refresh_dashboard();
            }

            // Right side: Reset + Refresh + Close
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if animated_button(ui, "Close", TEXT_DIM, "stats_close").clicked() {
                    self.view_mode = ViewMode::JobList;
                }
                ui.add_space(8.0);
                if animated_button(ui, "Refresh", ACCENT_CYAN, "stats_refresh").clicked() {
                    self.refresh_dashboard();
                }
                ui.add_space(8.0);
                if animated_button(ui, "Reset", ACCENT_RED, "stats_reset").clicked() {
                    self.stats_reset_confirm = true;
                }
            });
        });
    }
}
