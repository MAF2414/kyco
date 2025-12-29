//! Dashboard view rendering for KycoApp
//!
//! Single-page dashboard with summary cards, ring charts, and mode/chain table.

use eframe::egui::{self, RichText, ScrollArea};

use super::animations::animated_button;
use super::app::KycoApp;
use super::app_types::ViewMode;
use super::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, ACCENT_YELLOW, BG_HIGHLIGHT, BG_PRIMARY,
    BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::stats::{DashboardFilter, TimeRange};

mod charts;

impl KycoApp {
    pub(crate) fn render_stats(&mut self, ctx: &egui::Context) {
        // Auto-refresh every 5 seconds
        if self.stats_last_refresh.elapsed().as_secs() > 5 {
            self.refresh_dashboard();
        }

        // Reset confirmation dialog
        if self.stats_reset_confirm {
            self.render_reset_confirm_dialog(ctx);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_dashboard_header(ui);
                        ui.add_space(12.0);
                        self.render_summary_cards(ui);
                        ui.add_space(16.0);
                        self.render_ring_charts(ui);
                        ui.add_space(16.0);
                        self.render_mode_table(ui);
                        ui.add_space(16.0);
                        self.render_bottom_section(ui);
                    });
            });
    }

    fn render_reset_confirm_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Reset Statistics")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(egui::Frame::window(&ctx.style()).fill(BG_HIGHLIGHT).inner_margin(20.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("⚠️ Delete all statistics?").size(16.0).color(ACCENT_YELLOW));
                ui.add_space(8.0);
                ui.label(RichText::new("This will permanently delete all job, tool, and file statistics.").color(TEXT_PRIMARY));
                ui.label(RichText::new("This action cannot be undone.").color(ACCENT_RED));
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Cancel").color(TEXT_DIM)).clicked() {
                        self.stats_reset_confirm = false;
                    }
                    ui.add_space(16.0);
                    if ui.button(RichText::new("🗑 Delete All").color(ACCENT_RED)).clicked() {
                        if let Some(manager) = &self.stats_manager {
                            if manager.reset_all().is_ok() {
                                self.refresh_dashboard();
                            }
                        }
                        self.stats_reset_confirm = false;
                    }
                });
            });
    }

    fn render_dashboard_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("DASHBOARD").monospace().size(18.0).color(TEXT_PRIMARY));
            ui.add_space(16.0);

            // Time range selector
            ui.label(RichText::new("Range:").small().color(TEXT_DIM));
            egui::ComboBox::from_id_salt("stats_time_range")
                .selected_text(self.stats_time_range.label())
                .show_ui(ui, |ui| {
                    for range in [
                        TimeRange::Last15Minutes,
                        TimeRange::Last30Minutes,
                        TimeRange::Last1Hour,
                        TimeRange::Last8Hours,
                        TimeRange::Last1Day,
                        TimeRange::Last3Days,
                        TimeRange::Last7Days,
                        TimeRange::Last30Days,
                        TimeRange::Last90Days,
                        TimeRange::AllTime,
                    ] {
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

    fn refresh_dashboard(&mut self) {
        if let Some(manager) = &self.stats_manager {
            let filter = DashboardFilter {
                agent: self.stats_filter_agent.clone(),
                mode_or_chain: self.stats_filter_mode.clone(),
                workspace: self.stats_filter_workspace.clone(),
            };
            if let Ok(summary) = manager.query().get_dashboard(self.stats_time_range, &filter) {
                self.dashboard_summary = summary;
            }
        }
        self.stats_last_refresh = std::time::Instant::now();
    }

    fn render_summary_cards(&self, ui: &mut egui::Ui) {
        let s = &self.dashboard_summary;
        let spacing = 8.0;
        let row1_cards = 7.0;
        let row2_cards = 6.0;
        let row1_card_width = (ui.available_width() - spacing * (row1_cards - 1.0)) / row1_cards;
        let row2_card_width = (ui.available_width() - spacing * (row2_cards - 1.0)) / row2_cards;

        // Row 1: Jobs, Tokens, Cost, Bytes, Avg Time, Total Time, Wall Clock
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            summary_card_full(ui, "Jobs ✓", s.succeeded_jobs.current as u64, &s.succeeded_jobs, |v| v.to_string(), ACCENT_CYAN, row1_card_width, false);
            summary_card_full(ui, "Tokens", s.total_tokens.current as u64, &s.total_tokens, charts::format_tokens, ACCENT_CYAN, row1_card_width, false);
            summary_card_full_f64(ui, "Cost", s.total_cost.current, &s.total_cost, |v| format!("${:.2}", v), ACCENT_CYAN, row1_card_width, false);
            summary_card_full_f64(ui, "Bytes", s.total_bytes.current, &s.total_bytes, charts::format_bytes, ACCENT_CYAN, row1_card_width, false);
            summary_card_full_f64(ui, "Avg Time", s.avg_duration_ms.current, &s.avg_duration_ms, charts::format_duration, ACCENT_CYAN, row1_card_width, false);
            summary_card_full_f64(ui, "Total Time", s.total_duration_ms.current, &s.total_duration_ms, charts::format_duration, ACCENT_CYAN, row1_card_width, false);
            summary_card_full_f64(ui, "Wall Clock", s.wall_clock_ms.current, &s.wall_clock_ms, charts::format_duration, ACCENT_CYAN, row1_card_width, false);
        });

        ui.add_space(spacing);

        // Row 2: Input Tokens, Output Tokens, Cached, Tools, Files, Failed
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;
            summary_card_full(ui, "Input", s.input_tokens.current as u64, &s.input_tokens, charts::format_tokens, ACCENT_CYAN, row2_card_width, false);
            summary_card_full(ui, "Output", s.output_tokens.current as u64, &s.output_tokens, charts::format_tokens, ACCENT_CYAN, row2_card_width, false);
            summary_card_full(ui, "Cached", s.cached_tokens.current as u64, &s.cached_tokens, charts::format_tokens, ACCENT_CYAN, row2_card_width, false);
            summary_card_full(ui, "Tools", s.total_tool_calls.current as u64, &s.total_tool_calls, |v| v.to_string(), ACCENT_CYAN, row2_card_width, false);
            summary_card_full(ui, "Files", s.total_file_accesses.current as u64, &s.total_file_accesses, |v| v.to_string(), ACCENT_CYAN, row2_card_width, false);
            summary_card_full(ui, "Failed", s.failed_jobs.current as u64, &s.failed_jobs, |v| v.to_string(), ACCENT_RED, row2_card_width, true);
        });
    }

    fn render_ring_charts(&self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Agent ring chart
            cols[0].group(|ui| {
                ui.label(RichText::new("Agents").color(TEXT_PRIMARY));
                ui.add_space(4.0);
                charts::agent_ring_chart(ui, &self.dashboard_summary.agents, 120.0);
            });

            // Token ring chart
            cols[1].group(|ui| {
                ui.label(RichText::new("Token Types").color(TEXT_PRIMARY));
                ui.add_space(4.0);
                charts::token_ring_chart(ui, &self.dashboard_summary.tokens, 120.0);
            });
        });
    }

    fn render_mode_table(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("MODES & CHAINS").monospace().color(TEXT_PRIMARY));
        ui.add_space(4.0);

        egui::Frame::NONE.fill(BG_SECONDARY).corner_radius(4.0).inner_margin(8.0).show(ui, |ui| {
            if self.dashboard_summary.modes.is_empty() {
                ui.label(RichText::new("No mode data").small().color(TEXT_DIM));
                return;
            }

            egui::Grid::new("mode_stats_table")
                .num_columns(7)
                .spacing([16.0, 4.0])
                .min_col_width(50.0)
                .show(ui, |ui| {
                    // Header row
                    ui.label(RichText::new("Name").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Jobs").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Success").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Agent").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Avg Cost").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Avg Time").small().strong().color(TEXT_MUTED));
                    ui.label(RichText::new("Tokens (I/O/C)").small().strong().color(TEXT_MUTED));
                    ui.end_row();

                    // Data rows
                    for mode in &self.dashboard_summary.modes {
                        // Name
                        let name_display = if mode.name.len() > 14 {
                            format!("{}…", &mode.name[..13])
                        } else {
                            mode.name.clone()
                        };
                        ui.label(RichText::new(name_display).small().color(TEXT_PRIMARY));

                        // Jobs
                        ui.label(RichText::new(mode.total_jobs.to_string()).small().color(ACCENT_CYAN));

                        // Success rate
                        let success_color = if mode.success_rate() >= 80.0 { ACCENT_GREEN }
                            else if mode.success_rate() >= 50.0 { ACCENT_YELLOW }
                            else { ACCENT_RED };
                        ui.label(RichText::new(format!("{:.0}%", mode.success_rate())).small().color(success_color));

                        // Agent
                        let agent_color = if mode.primary_agent == "claude" { ACCENT_CYAN } else { ACCENT_PURPLE };
                        ui.label(RichText::new(&mode.primary_agent).small().color(agent_color));

                        // Avg cost
                        ui.label(RichText::new(format!("${:.3}", mode.avg_cost_usd)).small().color(ACCENT_GREEN));

                        // Avg time
                        ui.label(RichText::new(charts::format_duration(mode.avg_duration_ms as f64)).small().color(TEXT_DIM));

                        // Tokens I/O/C
                        let tokens_str = format!(
                            "{}/{}/{}",
                            charts::format_tokens(mode.tokens.input),
                            charts::format_tokens(mode.tokens.output),
                            charts::format_tokens(mode.tokens.total_cache())
                        );
                        ui.label(RichText::new(tokens_str).small().color(TEXT_DIM));
                        ui.end_row();
                    }
                });
        });
    }

    fn render_bottom_section(&self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            // Top Tools
            cols[0].label(RichText::new("Top Tools").color(TEXT_PRIMARY));
            egui::Frame::NONE.fill(BG_SECONDARY).corner_radius(4.0).inner_margin(8.0).show(&mut cols[0], |ui| {
                if self.dashboard_summary.top_tools.is_empty() {
                    ui.label(RichText::new("No tool data").small().color(TEXT_DIM));
                } else {
                    egui::Grid::new("top_tools_grid")
                        .num_columns(2)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            for (name, count) in &self.dashboard_summary.top_tools {
                                ui.label(RichText::new(name).small().color(TEXT_PRIMARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(count.to_string()).small().color(ACCENT_CYAN));
                                });
                                ui.end_row();
                            }
                        });
                }
            });

            // Top Files
            cols[1].label(RichText::new("Top Files").color(TEXT_PRIMARY));
            egui::Frame::NONE.fill(BG_SECONDARY).corner_radius(4.0).inner_margin(8.0).show(&mut cols[1], |ui| {
                if self.dashboard_summary.top_files.is_empty() {
                    ui.label(RichText::new("No file data").small().color(TEXT_DIM));
                } else {
                    egui::Grid::new("top_files_grid")
                        .num_columns(2)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            for (path, count) in &self.dashboard_summary.top_files {
                                let display = if path.len() > 30 {
                                    format!("…{}", &path[path.len() - 29..])
                                } else {
                                    path.clone()
                                };
                                ui.label(RichText::new(display).small().color(TEXT_PRIMARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new(count.to_string()).small().color(ACCENT_PURPLE));
                                });
                                ui.end_row();
                            }
                        });
                }
            });
        });
    }
}

// Helper functions for summary cards

fn summary_card_full<F>(
    ui: &mut egui::Ui,
    label: &str,
    value: u64,
    trend: &crate::stats::TrendValue,
    format: F,
    value_color: egui::Color32,
    width: f32,
    invert_trend: bool,
) where
    F: Fn(u64) -> String,
{
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(width - 24.0); // Account for inner margin
            ui.vertical(|ui| {
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                ui.label(RichText::new(format(value)).size(18.0).color(value_color));
                if invert_trend {
                    render_trend_inverted(ui, trend);
                } else {
                    render_trend(ui, trend);
                }
            });
        });
}

fn summary_card_full_f64<F>(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    trend: &crate::stats::TrendValue,
    format: F,
    value_color: egui::Color32,
    width: f32,
    invert_trend: bool,
) where
    F: Fn(f64) -> String,
{
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(width - 24.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                ui.label(RichText::new(format(value)).size(18.0).color(value_color));
                if invert_trend {
                    render_trend_inverted(ui, trend);
                } else {
                    render_trend(ui, trend);
                }
            });
        });
}

fn render_trend(ui: &mut egui::Ui, trend: &crate::stats::TrendValue) {
    let pct = trend.percent_change();
    if pct.abs() < 0.1 {
        ui.label(RichText::new("—").small().color(TEXT_DIM));
    } else {
        let (prefix, color) = if pct > 0.0 {
            ("▲", ACCENT_GREEN)
        } else {
            ("▼", ACCENT_RED)
        };
        ui.label(RichText::new(format!("{}{:.0}%", prefix, pct.abs())).small().color(color));
    }
}

fn render_trend_inverted(ui: &mut egui::Ui, trend: &crate::stats::TrendValue) {
    let pct = trend.percent_change();
    if pct.abs() < 0.1 {
        ui.label(RichText::new("—").small().color(TEXT_DIM));
    } else {
        // Inverted: up is bad (red), down is good (green)
        let (prefix, color) = if pct > 0.0 {
            ("▲", ACCENT_RED)
        } else {
            ("▼", ACCENT_GREEN)
        };
        ui.label(RichText::new(format!("{}{:.0}%", prefix, pct.abs())).small().color(color));
    }
}
