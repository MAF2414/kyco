//! Action buttons and permission handling for the detail panel

use std::collections::HashMap;

use eframe::egui::{self, RichText};

use crate::agent::bridge::PermissionMode;
use crate::config::Config;
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, STATUS_QUEUED, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::{Job, JobId, JobStatus, SdkType};

use super::types::DetailPanelAction;

/// Render action buttons and return any triggered action
pub(super) fn render_action_buttons(
    ui: &mut egui::Ui,
    job: &Job,
    continuation_prompt: &mut String,
    config: &Config,
    permission_mode_overrides: &HashMap<JobId, PermissionMode>,
) -> Option<DetailPanelAction> {
    let mut action = None;
    let current_job_id = job.id;
    let current_status = job.status;

    ui.horizontal(|ui| {
        match current_status {
            JobStatus::Pending => {
                if ui
                    .button(RichText::new("▶ Start").color(ACCENT_GREEN))
                    .clicked()
                {
                    action = Some(DetailPanelAction::Queue(current_job_id));
                }
            }
            JobStatus::Done => {
                // Multi-agent groups: offer comparison as the primary UI entrypoint.
                if let Some(group_id) = job.group_id {
                    if ui
                        .button(RichText::new("≍ Compare").color(ACCENT_CYAN))
                        .on_hover_text("Compare results from all agents in this group")
                        .clicked()
                    {
                        action = Some(DetailPanelAction::CompareGroup(group_id));
                    }
                }

                let base_branch = job.base_branch.as_deref().unwrap_or("current branch");
                let merge_hover = if job.group_id.is_some() {
                    format!(
                        "Merge this result into '{}' and clean up all group worktrees",
                        base_branch
                    )
                } else if job.git_worktree_path.is_some() {
                    format!("Merge this job's worktree into '{}'", base_branch)
                } else {
                    "No worktree: commit ALL current workspace changes".to_string()
                };

                if ui
                    .button(RichText::new("✓ Merge").color(ACCENT_GREEN))
                    .on_hover_text(merge_hover)
                    .clicked()
                {
                    action = Some(DetailPanelAction::Apply(current_job_id));
                }
                if ui
                    .button(RichText::new("✗ Reject").color(ACCENT_RED))
                    .clicked()
                {
                    action = Some(DetailPanelAction::Reject(current_job_id));
                }
                if ui.button(RichText::new("Δ Diff").color(TEXT_DIM)).clicked() {
                    action = Some(DetailPanelAction::ViewDiff(current_job_id));
                }
            }
            JobStatus::Running => {
                ui.label(RichText::new("⟳ Running...").color(STATUS_RUNNING));
                ui.add_space(8.0);

                if job.is_repl {
                    // REPL jobs: user can mark as complete when they're done in Terminal
                    if ui
                        .button(RichText::new("✓ Mark Complete").color(ACCENT_GREEN))
                        .on_hover_text("Mark this Terminal session as complete")
                        .clicked()
                    {
                        action = Some(DetailPanelAction::MarkComplete(current_job_id));
                    }
                } else {
                    // Print mode jobs: can be stopped/killed
                    if ui
                        .button(RichText::new("■ Stop").color(ACCENT_RED))
                        .on_hover_text("Stop this job")
                        .clicked()
                    {
                        action = Some(DetailPanelAction::Kill(current_job_id));
                    }
                }

                let is_claude_session = config
                    .get_agent_for_job(&job.agent_id, &job.mode)
                    .map(|a| a.sdk_type != SdkType::Codex)
                    .unwrap_or(job.agent_id != "codex");

                if is_claude_session {
                    ui.add_space(12.0);
                    ui.label(RichText::new("Permission").color(TEXT_MUTED));
                    ui.add_space(4.0);

                    let current_mode =
                        effective_permission_mode(job, config, permission_mode_overrides);
                    let mut desired_mode = current_mode;
                    let can_change = job.bridge_session_id.is_some();

                    ui.add_enabled_ui(can_change, |ui| {
                        // Use tuple-based ID to avoid format! allocation every frame
                        egui::ComboBox::from_id_salt(("permission_mode", current_job_id))
                            .selected_text(permission_mode_display(current_mode))
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut desired_mode,
                                    PermissionMode::Default,
                                    "Default (ask)",
                                );
                                ui.selectable_value(
                                    &mut desired_mode,
                                    PermissionMode::AcceptEdits,
                                    "Accept edits",
                                );
                                ui.selectable_value(
                                    &mut desired_mode,
                                    PermissionMode::Plan,
                                    "Plan",
                                );
                                ui.selectable_value(
                                    &mut desired_mode,
                                    PermissionMode::BypassPermissions,
                                    "Bypass permissions",
                                );
                            });
                    });

                    if can_change && desired_mode != current_mode {
                        action = Some(DetailPanelAction::SetPermissionMode(
                            current_job_id,
                            desired_mode,
                        ));
                    }
                }
            }
            JobStatus::Queued => {
                ui.label(RichText::new("◎ Queued").color(STATUS_QUEUED));
            }
            _ => {}
        }
    });

    // Session continuation UI for completed jobs with session ID
    if current_status == JobStatus::Done && job.bridge_session_id.is_some() {
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(
            RichText::new("Continue Session")
                .monospace()
                .color(ACCENT_CYAN),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(continuation_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .hint_text("Follow-up prompt...")
                    .desired_width(ui.available_width() - 80.0),
            );

            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let button_clicked = ui
                .button(RichText::new("▶ Send").color(ACCENT_GREEN))
                .clicked();

            if (submitted || button_clicked) && !continuation_prompt.is_empty() {
                let prompt = std::mem::take(continuation_prompt);
                action = Some(DetailPanelAction::Continue(current_job_id, prompt));
            }
        });
    }

    action
}

fn effective_permission_mode(
    job: &Job,
    config: &Config,
    overrides: &HashMap<JobId, PermissionMode>,
) -> PermissionMode {
    if let Some(mode) = overrides.get(&job.id).copied() {
        return mode;
    }

    config
        .get_agent_for_job(&job.agent_id, &job.mode)
        .map(|a| parse_claude_permission_mode(&a.permission_mode))
        .unwrap_or(PermissionMode::Default)
}

fn parse_claude_permission_mode(mode: &str) -> PermissionMode {
    match mode {
        "default" => PermissionMode::Default,
        "acceptEdits" | "accept_edits" | "accept-edits" => PermissionMode::AcceptEdits,
        "bypassPermissions" | "bypass_permissions" | "bypass-permissions" => {
            PermissionMode::BypassPermissions
        }
        "plan" => PermissionMode::Plan,
        _ => PermissionMode::Default,
    }
}

fn permission_mode_display(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Default => "Default (ask)",
        PermissionMode::AcceptEdits => "Accept edits",
        PermissionMode::BypassPermissions => "Bypass permissions",
        PermissionMode::Plan => "Plan",
    }
}
