//! Comparison popup for multi-agent results
//!
//! This popup allows users to compare results from multiple agents that ran
//! the same task in parallel, and select the best one to merge.

mod card;

use eframe::egui::{self, RichText, ScrollArea, Vec2};

use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, BG_PRIMARY, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::{AgentGroupId, AgentRunGroup, GroupStatus, Job, JobId};

use card::{render_agent_card, CardAction};

/// State for the comparison popup
pub struct ComparisonState {
    pub group: Option<AgentRunGroup>,
    pub jobs: Vec<Job>,
    pub selected_job_id: Option<JobId>,
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
    SelectJob(JobId),
    ViewDiff(JobId),
    /// Merge the selected job and cleanup other worktrees
    MergeAndClose,
    Cancel,
}

/// Render the comparison popup
///
/// Returns an action if the user interacted with the popup
pub fn render_comparison_popup(
    ctx: &egui::Context,
    state: &mut ComparisonState,
) -> Option<ComparisonAction> {
    if !state.show {
        return None;
    }

    let group = match state.group.as_ref() {
        Some(g) => g,
        None => return None,
    };
    let mut action = None;

    let num_agents = group.job_ids.len();
    let card_width = 180.0;
    let card_spacing = 16.0;
    let popup_width = (num_agents as f32 * card_width)
        + (num_agents.saturating_sub(1) as f32 * card_spacing)
        + 48.0;
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
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Compare Results: \"{}\"",
                        truncate(&group.prompt, 40)
                    ))
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
                ui.label(RichText::new(format!("Skill: {}", group.skill)).color(TEXT_DIM));
                ui.label(
                    RichText::new(format!("Target: {}", truncate(&group.target, 30)))
                        .color(TEXT_MUTED),
                );
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (idx, &job_id) in group.job_ids.iter().enumerate() {
                            let agent_name = group
                                .agent_names
                                .get(idx)
                                .map(|s| s.as_str())
                                .unwrap_or("unknown");
                            let job = state.jobs.iter().find(|j| j.id == job_id);
                            let is_selected = state.selected_job_id == Some(job_id);

                            if let Some(card_action) =
                                render_agent_card(ui, agent_name, job, is_selected)
                            {
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

            ui.horizontal(|ui| {
                let status_msg = match group.status {
                    GroupStatus::Running => "⏳ Waiting for agents to finish...",
                    GroupStatus::Comparing => "✓ All agents finished. Select the best result.",
                    GroupStatus::Selected => "★ Result selected. Click 'Merge & Close' to apply.",
                    GroupStatus::Merged => "✓ Changes merged.",
                    GroupStatus::Cancelled => "✗ Cancelled.",
                };
                ui.label(RichText::new(status_msg).color(TEXT_DIM));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_merge = state.selected_job_id.is_some()
                        && matches!(group.status, GroupStatus::Comparing | GroupStatus::Selected);

                    let merge_btn =
                        egui::Button::new(RichText::new("Merge & Close").color(if can_merge {
                            BG_PRIMARY
                        } else {
                            TEXT_MUTED
                        }))
                        .fill(if can_merge {
                            ACCENT_GREEN
                        } else {
                            BG_SECONDARY
                        });

                    if ui.add_enabled(can_merge, merge_btn).clicked() {
                        action = Some(ComparisonAction::MergeAndClose);
                    }

                    ui.add_space(8.0);

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

/// Truncate a string to a maximum number of characters (UTF-8 safe)
fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
