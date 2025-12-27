//! Main detail panel rendering

use eframe::egui::{self, RichText, ScrollArea};
use std::collections::HashMap;

use crate::agent::bridge::PermissionMode;
use crate::config::Config;
use crate::{AgentGroupId, ChainStepSummary, Job, JobId, JobStatus, LogEvent, SdkType};

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_HIGHLIGHT, BG_SECONDARY, STATUS_QUEUED,
    STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

use super::colors::{log_color, status_color};
use super::prompt::build_prompt_preview;
use crate::gui::diff::render_diff_content;

/// Actions that can be triggered from the detail panel
#[derive(Debug, Clone)]
pub enum DetailPanelAction {
    Queue(JobId),
    Apply(JobId),
    Reject(JobId),
    ViewDiff(JobId),
    /// Open the multi-agent comparison popup for this group
    CompareGroup(AgentGroupId),
    /// Stop/kill a running job
    Kill(JobId),
    /// Mark a REPL job as complete (user confirms they finished in Terminal)
    MarkComplete(JobId),
    /// Continue a session with a follow-up prompt
    Continue(JobId, String), // job_id, prompt
    /// Change permission mode for a running Claude session
    SetPermissionMode(JobId, PermissionMode),
}

/// UI filters for activity log display.
///
/// Defaults to showing only text events to keep the log readable.
#[derive(Debug, Clone)]
pub struct ActivityLogFilters {
    pub show_thought: bool,
    pub show_tool_call: bool,
    pub show_tool_output: bool,
    pub show_text: bool,
    pub show_error: bool,
    pub show_system: bool,
    pub show_permission: bool,
}

impl Default for ActivityLogFilters {
    fn default() -> Self {
        Self {
            show_thought: false,
            show_tool_call: false,
            show_tool_output: false,
            show_text: true,
            show_error: false,
            show_system: false,
            show_permission: false,
        }
    }
}

impl ActivityLogFilters {
    fn is_enabled(&self, kind: &crate::LogEventKind) -> bool {
        use crate::LogEventKind;
        match kind {
            LogEventKind::Thought => self.show_thought,
            LogEventKind::ToolCall => self.show_tool_call,
            LogEventKind::ToolOutput => self.show_tool_output,
            LogEventKind::Text => self.show_text,
            LogEventKind::Error => self.show_error,
            LogEventKind::System => self.show_system,
            LogEventKind::Permission => self.show_permission,
        }
    }

    fn selected_summary(&self) -> String {
        let mut selected = 0usize;
        let mut label: Option<&'static str> = None;

        let mut consider = |enabled: bool, name: &'static str| {
            if enabled {
                selected += 1;
                if label.is_none() {
                    label = Some(name);
                }
            }
        };

        consider(self.show_text, "Text");
        consider(self.show_tool_call, "Tool calls");
        consider(self.show_tool_output, "Tool output");
        consider(self.show_thought, "Thought");
        consider(self.show_system, "System");
        consider(self.show_error, "Error");
        consider(self.show_permission, "Permission");

        match (selected, label) {
            (0, _) => "None".to_string(),
            (1, Some(name)) => name.to_string(),
            (n, Some(name)) => format!("{name} +{}", n.saturating_sub(1)),
            _ => "Selected".to_string(),
        }
    }
}

/// State required for rendering the detail panel
pub struct DetailPanelState<'a> {
    pub selected_job_id: Option<u64>,
    pub cached_jobs: &'a [Job],
    pub logs: &'a [LogEvent],
    pub config: &'a Config,
    pub log_scroll_to_bottom: bool,
    pub activity_log_filters: &'a mut ActivityLogFilters,
    /// Input buffer for session continuation prompt
    pub continuation_prompt: &'a mut String,
    /// Markdown cache for rendering agent responses
    pub commonmark_cache: &'a mut egui_commonmark::CommonMarkCache,
    /// Current Claude permission mode overrides per job
    pub permission_mode_overrides: &'a HashMap<JobId, PermissionMode>,
    /// Diff content for the selected job (if available)
    pub diff_content: Option<&'a str>,
}

/// Render the detail panel and return any action triggered by the user
pub fn render_detail_panel(
    ui: &mut egui::Ui,
    state: &mut DetailPanelState<'_>,
) -> Option<DetailPanelAction> {
    let mut action: Option<DetailPanelAction> = None;

    let available_width = ui.available_width();
    let available_height = ui.available_height();

    render_header(ui);

    if let Some(job_id) = state.selected_job_id {
        if let Some(job) = state.cached_jobs.iter().find(|j| j.id == job_id) {
            // Wrap everything in a ScrollArea to ensure we never overflow
            ScrollArea::vertical()
                .id_salt(("detail_panel_scroll", job.id))
                .auto_shrink([false, false])
                .max_height(available_height - 30.0) // Reserve space for header
                .show(ui, |ui| {
                    ui.set_min_width(available_width - 16.0);

                    render_job_info(ui, job);
                    render_result_section(ui, job, state.commonmark_cache);

                    ui.add_space(8.0);

                    action = render_action_buttons(
                        ui,
                        job,
                        state.continuation_prompt,
                        state.config,
                        state.permission_mode_overrides,
                    );

                    ui.add_space(8.0);
                    ui.separator();

                    if job.chain_name.is_some() || !job.chain_step_history.is_empty() {
                        render_chain_progress_section_with_height(ui, job, state.commonmark_cache, available_width);
                        ui.add_space(4.0);
                    }

                    render_prompt_section_collapsible(ui, job, state.config);

                    ui.add_space(4.0);

                    if let Some(diff_content) = state.diff_content {
                        render_diff_section_inline(ui, diff_content, available_width);
                    } else {
                        ui.add_space(8.0);
                        egui::Frame::NONE
                            .fill(BG_SECONDARY)
                            .corner_radius(4.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.set_min_width(available_width - 48.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("No diff available").color(TEXT_MUTED));
                                    ui.label(RichText::new("Diff will appear here when the job completes with changes").color(TEXT_DIM).small());
                                });
                            });
                    }

                    ui.add_space(4.0);

                    render_activity_log_inline(
                        ui,
                        job,
                        state.logs,
                        state.log_scroll_to_bottom,
                        available_width,
                        state.commonmark_cache,
                        state.activity_log_filters,
                    );
                });
        } else {
            ui.label(RichText::new("Job not found").color(TEXT_MUTED));
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Select a job to view details").color(TEXT_MUTED));
        });
    }

    action
}

fn render_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("DETAILS").monospace().color(TEXT_PRIMARY));
    });
    ui.add_space(4.0);
    ui.separator();
}

fn render_job_info(ui: &mut egui::Ui, job: &Job) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Job #{}", job.id))
                .monospace()
                .color(TEXT_PRIMARY),
        );
        let color = status_color(job.status);
        ui.label(
            RichText::new(format!("[{}]", job.status))
                .monospace()
                .color(color),
        );
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.mode).color(TEXT_PRIMARY));
        ui.label(RichText::new("Agent:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.agent_id).color(ACCENT_CYAN));
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Target:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.target).color(TEXT_DIM));
    });

    if let Some(desc) = &job.description {
        ui.add_space(4.0);
        ui.label(RichText::new(desc).color(TEXT_DIM));
    }
}

/// Apply CRT theme visuals for markdown rendering
#[inline]
fn apply_markdown_theme(ui: &mut egui::Ui) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.override_text_color = Some(TEXT_DIM);
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.hyperlink_color = ACCENT_CYAN;
    visuals.code_bg_color = BG_HIGHLIGHT;
    visuals.extreme_bg_color = BG_HIGHLIGHT;
    visuals.widgets.active.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;
}

#[inline]
fn apply_markdown_theme_with_text_color(ui: &mut egui::Ui, text_color: egui::Color32) {
    apply_markdown_theme(ui);
    ui.style_mut().visuals.override_text_color = Some(text_color);
}

fn render_markdown_inline_colored(
    ui: &mut egui::Ui,
    text: &str,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    text_color: egui::Color32,
) {
    ui.scope(|ui| {
        apply_markdown_theme_with_text_color(ui, text_color);
        egui_commonmark::CommonMarkViewer::new().show(ui, commonmark_cache, text);
    });
}

/// Render markdown content with themed scroll area
fn render_markdown_scroll(
    ui: &mut egui::Ui,
    text: &str,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
) {
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.scope(|ui| {
                apply_markdown_theme(ui);
                egui_commonmark::CommonMarkViewer::new().show(ui, commonmark_cache, text);
            });
        });
}

/// Render result section (from YAML block or raw text)
fn render_result_section(
    ui: &mut egui::Ui,
    job: &Job,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
) {
    let response_text = job
        .full_response
        .as_deref()
        .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()));

    if let Some(result) = &job.result {
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                let has_structured =
                    result.title.is_some() || result.status.is_some() || result.details.is_some();

                if has_structured {
                    if let Some(title) = &result.title {
                        ui.label(RichText::new(title).monospace().color(TEXT_PRIMARY));
                    }

                    if let Some(details) = &result.details {
                        ui.add_space(4.0);
                        ui.label(RichText::new(details).color(TEXT_DIM));
                    }

                    if let Some(summary) = &result.summary {
                        if !summary.is_empty() {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                            ui.label(RichText::new(summary).color(TEXT_DIM).small());
                        }
                    }

                    ui.add_space(8.0);
                    render_stats_bar(ui, job, result);
                }

                if let Some(text) = response_text {
                    if has_structured {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                    ui.add_space(4.0);

                    render_markdown_scroll(ui, text, commonmark_cache);

                    // Still show stats if we didn't render the structured stats bar
                    if !has_structured {
                        if let Some(stats) = &job.stats {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if stats.files_changed > 0 {
                                    ui.label(
                                        RichText::new(format!("{} files", stats.files_changed))
                                            .color(TEXT_MUTED),
                                    );
                                    ui.add_space(8.0);
                                }
                                if let Some(duration) = job.duration_string() {
                                    ui.label(RichText::new(duration).color(TEXT_MUTED));
                                }
                            });
                        }
                    }
                }
            });
    } else if let Some(text) = response_text {
        // No parsed result, but we have a full response to display.
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                ui.add_space(4.0);

                render_markdown_scroll(ui, text, commonmark_cache);
            });
    } else if let Some(stats) = &job.stats {
        // Show just stats if no result block but we have timing/files
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if stats.files_changed > 0 {
                ui.label(
                    RichText::new(format!("{} files changed", stats.files_changed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
            if let Some(duration) = job.duration_string() {
                ui.label(RichText::new(format!("⏱ {}", duration)).color(TEXT_MUTED));
            }
        });
    }
}

fn render_stats_bar(ui: &mut egui::Ui, job: &Job, result: &crate::JobResult) {
    ui.horizontal(|ui| {
        if let Some(status) = &result.status {
            let result_status_color = match status.as_str() {
                "success" => ACCENT_GREEN,
                "partial" => STATUS_RUNNING,
                "failed" => ACCENT_RED,
                _ => TEXT_MUTED,
            };
            ui.label(RichText::new(format!("● {}", status)).color(result_status_color));
            ui.add_space(8.0);
        }

        if let Some(stats) = &job.stats {
            if stats.files_changed > 0 {
                ui.label(RichText::new(format!("{} files", stats.files_changed)).color(TEXT_MUTED));
                ui.add_space(8.0);
            }
            if stats.lines_added > 0 || stats.lines_removed > 0 {
                ui.label(
                    RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
        }

        if let Some(duration) = job.duration_string() {
            ui.label(RichText::new(duration).color(TEXT_MUTED));
        }
    });
}

/// Render action buttons and return any triggered action
fn render_action_buttons(
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

/// Get static string label for log event kind (avoids format! allocation per frame)
#[inline]
fn log_kind_label(kind: &crate::LogEventKind) -> &'static str {
    use crate::LogEventKind;
    match kind {
        LogEventKind::Thought => "[thought]",
        LogEventKind::ToolCall => "[tool]",
        LogEventKind::ToolOutput => "[output]",
        LogEventKind::Text => "[text]",
        LogEventKind::Error => "[error]",
        LogEventKind::System => "[system]",
        LogEventKind::Permission => "[permission]",
    }
}

// ============================================================================
// Collapsible sections for clean layout
// ============================================================================

/// Render prompt section with collapsible header (collapsed by default)
fn render_prompt_section_collapsible(ui: &mut egui::Ui, job: &Job, config: &Config) {
    use std::borrow::Cow;

    let (prompt_text, prompt_label): (Cow<'_, str>, &str) = match &job.sent_prompt {
        Some(prompt) => (Cow::Borrowed(prompt.as_str()), "SENT PROMPT"),
        None => (
            Cow::Owned(build_prompt_preview(job, config)),
            "PROMPT PREVIEW",
        ),
    };

    let line_count = prompt_text.lines().count();

    egui::CollapsingHeader::new(
        RichText::new(format!("{} ({} lines)", prompt_label, line_count))
            .monospace()
            .color(TEXT_MUTED),
    )
    .default_open(false)
    .show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt("prompt_scroll")
            .auto_shrink([false, false])
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut prompt_text.as_ref())
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_DIM)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
    });
}

/// Render diff section inline (no inner scroll - parent handles scrolling)
fn render_diff_section_inline(ui: &mut egui::Ui, diff_content: &str, available_width: f32) {
    let added = diff_content
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();
    let removed = diff_content
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    egui::CollapsingHeader::new(RichText::new("DIFF").monospace().color(TEXT_PRIMARY))
        .id_salt("diff_section")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if added > 0 {
                    ui.label(
                        RichText::new(format!("+{}", added))
                            .color(ACCENT_GREEN)
                            .small(),
                    );
                }
                if removed > 0 {
                    ui.label(
                        RichText::new(format!("-{}", removed))
                            .color(ACCENT_RED)
                            .small(),
                    );
                }
            });
            ui.add_space(4.0);

            egui::Frame::NONE
                .fill(BG_SECONDARY)
                .corner_radius(4.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_min_width(available_width - 24.0);
                    render_diff_content(ui, diff_content);
                });
        });
}

/// Render activity log section inline (no inner scroll - parent handles scrolling)
fn render_activity_log_inline(
    ui: &mut egui::Ui,
    job: &Job,
    logs: &[LogEvent],
    _scroll_to_bottom: bool,
    available_width: f32,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    filters: &mut ActivityLogFilters,
) {
    let total_log_count = job.log_events.len()
        + logs
            .iter()
            .filter(|e| e.job_id.is_none() || e.job_id == Some(job.id))
            .count();

    let shown_log_count = job
        .log_events
        .iter()
        .filter(|e| filters.is_enabled(&e.kind))
        .count()
        + logs
            .iter()
            .filter(|e| {
                (e.job_id.is_none() || e.job_id == Some(job.id)) && filters.is_enabled(&e.kind)
            })
            .count();

    // Use stable id_salt based on job ID to prevent state reset when log count changes
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "ACTIVITY LOG ({}/{})",
            shown_log_count, total_log_count
        ))
        .monospace()
        .color(TEXT_MUTED),
    )
    .id_salt(("activity_log", job.id))
    .default_open(false)
    .show(ui, |ui| {
        ui.set_min_width(available_width - 16.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Show:").small().color(TEXT_MUTED));
            egui::ComboBox::from_id_salt(("activity_log_filters", job.id))
                .selected_text(
                    RichText::new(filters.selected_summary())
                        .small()
                        .color(TEXT_PRIMARY),
                )
                .width(140.0)
                .show_ui(ui, |ui| {
                    ui.checkbox(&mut filters.show_text, "Text");
                    ui.checkbox(&mut filters.show_tool_call, "Tool calls");
                    ui.checkbox(&mut filters.show_tool_output, "Tool output");
                    ui.checkbox(&mut filters.show_thought, "Thought");
                    ui.checkbox(&mut filters.show_system, "System");
                    ui.checkbox(&mut filters.show_error, "Error");
                    ui.checkbox(&mut filters.show_permission, "Permission");

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Only text").clicked() {
                            *filters = ActivityLogFilters::default();
                        }
                        if ui.button("All").clicked() {
                            filters.show_text = true;
                            filters.show_tool_call = true;
                            filters.show_tool_output = true;
                            filters.show_thought = true;
                            filters.show_system = true;
                            filters.show_error = true;
                            filters.show_permission = true;
                        }
                    });
                });
        });

        ui.add_space(6.0);

        // Show job-specific logs first
        for event in &job.log_events {
            if !filters.is_enabled(&event.kind) {
                continue;
            }

            let color = log_color(&event.kind);
            render_activity_log_event(ui, event, commonmark_cache, color);
        }

        // Then show global logs filtered by job_id
        for event in logs {
            if (event.job_id.is_none() || event.job_id == Some(job.id))
                && filters.is_enabled(&event.kind)
            {
                let color = log_color(&event.kind);
                render_activity_log_event(ui, event, commonmark_cache, color);
            }
        }
    });
}

fn render_activity_log_event(
    ui: &mut egui::Ui,
    event: &LogEvent,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    color: egui::Color32,
) {
    let text = event.content.as_deref().unwrap_or(event.summary.as_str());
    let render_markdown = matches!(event.kind, crate::LogEventKind::Text);

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(log_kind_label(&event.kind))
                        .monospace()
                        .color(TEXT_MUTED),
                );
                if let Some(tool) = event.tool_name.as_deref() {
                    ui.label(RichText::new(tool).monospace().small().color(TEXT_MUTED));
                }
            });

            ui.add_space(2.0);

            if render_markdown {
                render_markdown_inline_colored(ui, text, commonmark_cache, color);
            } else {
                ui.label(RichText::new(text).color(color));
            }
        });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);
}

// ============================================================================
// Chain Progress Section
// ============================================================================

/// Render chain progress section inline (no inner scroll - parent handles scrolling)
fn render_chain_progress_section_with_height(
    ui: &mut egui::Ui,
    job: &Job,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    available_width: f32,
) {
    let chain_name = job.chain_name.as_deref().unwrap_or("Chain");
    let total_steps = job
        .chain_total_steps
        .unwrap_or(job.chain_step_history.len());
    let current_step = job.chain_current_step.unwrap_or(0);
    let completed_steps = job
        .chain_step_history
        .iter()
        .filter(|s| !s.skipped && s.success)
        .count();
    let is_running = job.status == JobStatus::Running;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("⛓ {}", chain_name))
                .monospace()
                .color(ACCENT_CYAN),
        );
        ui.add_space(8.0);

        if is_running && total_steps > 0 {
            ui.label(
                RichText::new(format!("Step {}/{}", current_step + 1, total_steps))
                    .color(STATUS_RUNNING)
                    .small(),
            );
        } else if !job.chain_step_history.is_empty() {
            let status_color = if job.status == JobStatus::Done {
                ACCENT_GREEN
            } else if job.status == JobStatus::Failed {
                ACCENT_RED
            } else {
                TEXT_MUTED
            };
            ui.label(
                RichText::new(format!("{}/{} completed", completed_steps, total_steps))
                    .color(status_color)
                    .small(),
            );
        }
    });

    ui.add_space(4.0);

    if total_steps > 0 {
        let progress = if is_running {
            current_step as f32 / total_steps as f32
        } else {
            completed_steps as f32 / total_steps as f32
        };

        let progress_bar = egui::ProgressBar::new(progress)
            .fill(if is_running {
                STATUS_RUNNING
            } else if job.status == JobStatus::Done {
                ACCENT_GREEN
            } else {
                ACCENT_RED
            })
            .animate(is_running);
        ui.add_sized([available_width - 16.0, 8.0], progress_bar);
    }

    ui.add_space(8.0);

    if !job.chain_step_history.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new(format!("CHAIN STEPS ({})", job.chain_step_history.len()))
                .monospace()
                .color(TEXT_MUTED),
        )
        .id_salt(("chain_steps", job.id))
        .default_open(true)
        .show(ui, |ui| {
            ui.set_min_width(available_width - 16.0);
            for step in &job.chain_step_history {
                render_chain_step_full_width(
                    ui,
                    step,
                    job.id,
                    commonmark_cache,
                    available_width - 24.0,
                );
                ui.add_space(4.0);
            }
        });
    }
}

/// Render a single chain step with explicit width (no inner scroll)
fn render_chain_step_full_width(
    ui: &mut egui::Ui,
    step: &ChainStepSummary,
    job_id: u64,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    width: f32,
) {
    let (status_icon, status_color) = if step.skipped {
        ("○", TEXT_MUTED)
    } else if step.success {
        ("✓", ACCENT_GREEN)
    } else {
        ("✗", ACCENT_RED)
    };

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(width);

            ui.horizontal(|ui| {
                ui.label(RichText::new(status_icon).color(status_color));
                ui.label(
                    RichText::new(format!("{}. {}", step.step_index + 1, step.mode))
                        .monospace()
                        .color(TEXT_PRIMARY),
                );
                if step.skipped {
                    ui.label(RichText::new("(skipped)").color(TEXT_MUTED).small());
                } else if step.files_changed > 0 {
                    ui.label(
                        RichText::new(format!("{} files", step.files_changed))
                            .color(TEXT_MUTED)
                            .small(),
                    );
                }
            });

            if let Some(title) = &step.title {
                ui.label(RichText::new(title).color(TEXT_DIM));
            }

            if let Some(error) = &step.error {
                ui.label(
                    RichText::new(format!("Error: {}", error))
                        .color(ACCENT_RED)
                        .small(),
                );
            }

            if let Some(summary) = &step.summary {
                ui.add_space(4.0);
                ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                // Truncate long summaries safely at character boundary
                let truncated = if summary.chars().count() > 200 {
                    let mut end = summary
                        .char_indices()
                        .nth(200)
                        .map(|(i, _)| i)
                        .unwrap_or(summary.len());
                    // Ensure we don't split in the middle of a grapheme cluster
                    while !summary.is_char_boundary(end) && end > 0 {
                        end -= 1;
                    }
                    format!("{}...", &summary[..end])
                } else {
                    summary.clone()
                };
                ui.label(RichText::new(truncated).color(TEXT_DIM).small());
            }

            if let Some(response) = &step.full_response {
                ui.add_space(4.0);
                egui::CollapsingHeader::new(
                    RichText::new("Full Response").small().color(TEXT_MUTED),
                )
                .id_salt(("step_response_fw", job_id, step.step_index))
                .default_open(false)
                .show(ui, |ui| {
                    ui.set_min_width(width - 16.0);
                    ui.scope(|ui| {
                        apply_markdown_theme(ui);
                        egui_commonmark::CommonMarkViewer::new().show(
                            ui,
                            commonmark_cache,
                            response,
                        );
                    });
                });
            }
        });
}
