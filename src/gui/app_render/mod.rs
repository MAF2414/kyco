//! Render delegation methods for KycoApp
//!
//! Contains methods that delegate to specialized render modules.

mod config_views;
mod popups;

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::jobs;
use super::theme::BG_PRIMARY;
use eframe::egui;

impl KycoApp {
    /// Render the main content based on current view mode
    pub(crate) fn render_view_mode(&mut self, ctx: &egui::Context) {
        match self.view_mode {
            ViewMode::JobList => {
                egui::SidePanel::left("job_list")
                    .default_width(280.0)
                    .min_width(280.0)
                    .max_width(280.0)
                    .resizable(false)
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_job_list(ui);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_detail_panel(ui);
                    });
            }
            ViewMode::SelectionPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_selection_popup(ctx);
            }
            ViewMode::BatchPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_batch_popup(ctx);
            }
            ViewMode::DiffView => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_diff_popup(ctx);
            }
            ViewMode::ApplyConfirmPopup => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_apply_confirm_popup(ctx);
            }
            ViewMode::ComparisonPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_comparison_popup(ctx);
            }
            ViewMode::Settings => {
                self.render_settings(ctx);
                // Apply voice config changes to VoiceManager after settings are saved
                if self.voice_config_changed {
                    self.voice_config_changed = false;
                    self.apply_voice_config();
                }
            }
            ViewMode::Modes => {
                self.render_modes(ctx);
            }
            ViewMode::Agents => {
                self.render_agents(ctx);
            }
            ViewMode::Chains => {
                self.render_chains(ctx);
            }
            ViewMode::Stats => {
                self.render_stats(ctx);
            }
            ViewMode::Achievements => {
                self.render_achievements(ctx);
            }
        }
    }

    /// Render the job list panel
    pub(crate) fn render_job_list(&mut self, ui: &mut egui::Ui) {
        let action = jobs::render_job_list(
            ui,
            &self.cached_jobs,
            &mut self.selected_job_id,
            &mut self.job_list_filter,
        );

        // Handle actions
        match action {
            jobs::JobListAction::DeleteJob(job_id) => {
                self.delete_job(job_id);
            }
            jobs::JobListAction::DeleteAllFinished => {
                self.delete_all_finished_jobs();
            }
            jobs::JobListAction::None => {}
        }
    }

    /// Render the detail panel
    pub(crate) fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        use super::detail_panel::{DetailPanelAction, DetailPanelState, render_detail_panel};

        let action = {
            let Ok(config) = self.config.read() else {
                ui.label("Config unavailable");
                return;
            };
            let mut state = DetailPanelState {
                selected_job_id: self.selected_job_id,
                cached_jobs: &self.cached_jobs,
                logs: &self.logs,
                config: &config,
                log_scroll_to_bottom: self.log_scroll_to_bottom,
                activity_log_filters: &mut self.activity_log_filters,
                continuation_prompt: &mut self.continuation_prompt,
                commonmark_cache: &mut self.commonmark_cache,
                permission_mode_overrides: &self.permission_mode_overrides,
                diff_content: self.inline_diff_content.as_deref(),
            };

            render_detail_panel(ui, &mut state)
        };

        if let Some(action) = action {
            match action {
                DetailPanelAction::Queue(job_id) => self.queue_job(job_id),
                DetailPanelAction::Apply(job_id) => self.apply_job(job_id),
                DetailPanelAction::Reject(job_id) => self.reject_job(job_id),
                DetailPanelAction::CompareGroup(group_id) => self.open_comparison_popup(group_id),
                DetailPanelAction::Continue(job_id, prompt) => {
                    self.continue_job_session(job_id, prompt);
                }
                DetailPanelAction::ViewDiff(job_id) => {
                    self.open_job_diff(job_id, ViewMode::JobList)
                }
                DetailPanelAction::Kill(job_id) => self.kill_job(job_id),
                DetailPanelAction::MarkComplete(job_id) => self.mark_job_complete(job_id),
                DetailPanelAction::SetPermissionMode(job_id, mode) => {
                    self.set_job_permission_mode(job_id, mode);
                }
            }
        }
    }

    /// Render the diff view popup
    pub(crate) fn render_diff_popup(&mut self, ctx: &egui::Context) {
        if super::diff::render_diff_popup(ctx, &self.diff_state) {
            self.view_mode = self.diff_return_view;
            self.diff_state.clear();
        }
    }
}
