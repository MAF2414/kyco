//! Comparison popup for multi-agent results
//!
//! This popup allows users to compare results from multiple agents that ran
//! the same task in parallel, and select the best one to merge.

use eframe::egui::{self, RichText, ScrollArea, Vec2};

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_HIGHLIGHT, BG_PRIMARY, BG_SECONDARY, BG_SELECTED,
    STATUS_DONE, STATUS_FAILED, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::{AgentGroupId, AgentRunGroup, GroupStatus, Job, JobId, JobStatus};

/// State for the comparison popup
pub struct ComparisonState {
    /// The group being compared
    pub group: Option<AgentRunGroup>,
    /// Jobs in the group (for display)
    pub jobs: Vec<Job>,
    /// Currently selected job ID
    pub selected_job_id: Option<JobId>,
    /// Whether to show the popup
    pub show: bool,
}

impl Default for ComparisonState {
    fn default() -> Self {
        Self {
            group: None,
            jobs: Vec::new(),
            selected_job_id: None,
            show: false,
        }
    }
}

impl ComparisonState {
    /// Open the comparison popup for a group
    pub fn open(&mut self, group: AgentRunGroup, jobs: Vec<Job>) {
        self.selected_job_id = group.selected_job;
        self.group = Some(group);
        self.jobs = jobs;
        self.show = true;
    }

    /// Close the comparison popup
    pub fn close(&mut self) {
        self.show = false;
        self.group = None;
        self.jobs.clear();
        self.selected_job_id = None;
    }

    /// Get the current group ID
    pub fn group_id(&self) -> Option<AgentGroupId> {
        self.group.as_ref().map(|g| g.id)
    }
}

/// Actions that can be returned from the comparison popup
pub enum ComparisonAction {
    /// User selected a job
    SelectJob(JobId),
    /// User wants to view the diff for a job
    ViewDiff(JobId),
    /// User wants to merge the selected job and cleanup
    MergeAndClose,
    /// User cancelled/closed the popup
    Cancel,
}

/// Render the comparison popup
///
/// Returns an action if the user interacted with the popup
pub fn render_comparison_popup(
    ctx: &egui::Context,
    state: &mut ComparisonState,
) -> Option<ComparisonAction> {
    if !state.show || state.group.is_none() {
        return None;
    }

    let group = state.group.as_ref().unwrap();
    let mut action = None;

    // Calculate popup size based on number of agents
    let num_agents = group.job_ids.len();
    let card_width = 180.0;
    let card_spacing = 16.0;
    let popup_width = (num_agents as f32 * card_width) + ((num_agents - 1) as f32 * card_spacing) + 48.0;
    let popup_width = popup_width.max(400.0).min(900.0);

    egui::Window::new("Compare Agent Results")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(popup_width, 450.0))
        .frame(
            egui::Frame::default()
                .fill(BG_PRIMARY)
                .stroke(egui::Stroke::new(2.0, ACCENT_CYAN))
                .inner_margin(16.0)
                .corner_radius(8.0),
        )
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Compare Results: \"{}\"", truncate(&group.prompt, 40)))
                        .color(TEXT_PRIMARY)
                        .size(16.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("✕").color(TEXT_DIM)))
                        .clicked()
                    {
                        action = Some(ComparisonAction::Cancel);
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Mode: {}", group.mode)).color(TEXT_DIM));
                ui.label(RichText::new(format!("Target: {}", truncate(&group.target, 30))).color(TEXT_MUTED));
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Agent cards in a horizontal scroll area
            ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (idx, &job_id) in group.job_ids.iter().enumerate() {
                            let agent_name = group.agent_names.get(idx).map(|s| s.as_str()).unwrap_or("unknown");
                            let job = state.jobs.iter().find(|j| j.id == job_id);
                            let is_selected = state.selected_job_id == Some(job_id);

                            if let Some(card_action) = render_agent_card(ui, agent_name, job, is_selected) {
                                match card_action {
                                    CardAction::Select => {
                                        state.selected_job_id = Some(job_id);
                                        action = Some(ComparisonAction::SelectJob(job_id));
                                    }
                                    CardAction::ViewDiff => {
                                        action = Some(ComparisonAction::ViewDiff(job_id));
                                    }
                                }
                            }

                            if idx < group.job_ids.len() - 1 {
                                ui.add_space(card_spacing);
                            }
                        }
                    });
                });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Footer with buttons
            ui.horizontal(|ui| {
                // Status message
                let status_msg = match group.status {
                    GroupStatus::Running => "⏳ Waiting for agents to finish...",
                    GroupStatus::Comparing => "✓ All agents finished. Select the best result.",
                    GroupStatus::Selected => "★ Result selected. Click 'Merge & Close' to apply.",
                    GroupStatus::Merged => "✓ Changes merged.",
                    GroupStatus::Cancelled => "✗ Cancelled.",
                };
                ui.label(RichText::new(status_msg).color(TEXT_DIM));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Merge & Close button (only enabled when a job is selected)
                    let can_merge = state.selected_job_id.is_some()
                        && matches!(group.status, GroupStatus::Comparing | GroupStatus::Selected);

                    let merge_btn = egui::Button::new(
                        RichText::new("Merge & Close")
                            .color(if can_merge { BG_PRIMARY } else { TEXT_MUTED }),
                    )
                    .fill(if can_merge { ACCENT_GREEN } else { BG_SECONDARY });

                    if ui.add_enabled(can_merge, merge_btn).clicked() {
                        action = Some(ComparisonAction::MergeAndClose);
                    }

                    ui.add_space(8.0);

                    // Cancel button
                    if ui
                        .add(egui::Button::new(RichText::new("Cancel").color(TEXT_DIM)))
                        .clicked()
                    {
                        action = Some(ComparisonAction::Cancel);
                    }
                });
            });
        });

    action
}

/// Actions from an individual agent card
enum CardAction {
    Select,
    ViewDiff,
}

/// Render a single agent card
fn render_agent_card(
    ui: &mut egui::Ui,
    agent_name: &str,
    job: Option<&Job>,
    is_selected: bool,
) -> Option<CardAction> {
    let mut action = None;

    let bg_color = if is_selected { BG_SELECTED } else { BG_SECONDARY };
    let border_color = if is_selected { ACCENT_CYAN } else { BG_HIGHLIGHT };

    egui::Frame::default()
        .fill(bg_color)
        .stroke(egui::Stroke::new(2.0, border_color))
        .inner_margin(12.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.set_max_width(180.0);
            ui.set_min_height(220.0);

            // Agent name header
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(agent_name)
                        .color(TEXT_PRIMARY)
                        .size(14.0)
                        .strong(),
                );
                if is_selected {
                    ui.label(RichText::new("★").color(ACCENT_CYAN));
                }
            });

            ui.add_space(8.0);

            if let Some(job) = job {
                // Status indicator
                let (status_text, status_color) = match job.status {
                    JobStatus::Running => ("⟳ Running...", STATUS_RUNNING),
                    JobStatus::Done => ("✓ Done", STATUS_DONE),
                    JobStatus::Failed => ("✗ Failed", STATUS_FAILED),
                    JobStatus::Pending => ("○ Pending", TEXT_MUTED),
                    JobStatus::Queued => ("~ Queued", TEXT_DIM),
                    JobStatus::Rejected => ("- Rejected", ACCENT_RED),
                    JobStatus::Merged => ("> Merged", ACCENT_GREEN),
                };
                ui.label(RichText::new(status_text).color(status_color));

                ui.add_space(8.0);

                // Stats (if available)
                if let Some(stats) = &job.stats {
                    ui.label(
                        RichText::new(format!("{} files", stats.files_changed))
                            .color(TEXT_DIM)
                            .small(),
                    );
                    ui.label(
                        RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed))
                            .color(TEXT_DIM)
                            .small(),
                    );
                }

                // Duration
                if let Some(duration_str) = job.duration_string() {
                    ui.label(RichText::new(duration_str).color(TEXT_MUTED).small());
                }

                ui.add_space(8.0);

                // Result summary (if available)
                if let Some(result) = &job.result {
                    if let Some(title) = &result.title {
                        ui.label(
                            RichText::new(truncate(title, 25))
                                .color(TEXT_DIM)
                                .small(),
                        );
                    }
                }

                // Error message (if failed)
                if job.status == JobStatus::Failed {
                    if let Some(error) = &job.error_message {
                        ui.label(
                            RichText::new(truncate(error, 30))
                                .color(ACCENT_RED)
                                .small(),
                        );
                    }
                }

                ui.add_space(8.0);

                // Action buttons (only for completed jobs)
                if job.status == JobStatus::Done {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("View Diff").color(TEXT_DIM).small())
                                    .small(),
                            )
                            .clicked()
                        {
                            action = Some(CardAction::ViewDiff);
                        }
                    });

                    if !is_selected {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Select")
                                        .color(BG_PRIMARY)
                                        .small(),
                                )
                                .fill(ACCENT_CYAN)
                                .small(),
                            )
                            .clicked()
                        {
                            action = Some(CardAction::Select);
                        }
                    } else {
                        ui.label(RichText::new("★ Selected").color(ACCENT_CYAN).small());
                    }
                }
            } else {
                // No job data available
                ui.label(RichText::new("No data").color(TEXT_MUTED));
            }
        });

    action
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
