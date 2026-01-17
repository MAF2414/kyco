//! Unified Board - Jobs + Findings Kanban View
//!
//! Displays both jobs and findings in a combined Kanban-style board
//! with visual connections between jobs and the findings they produce.

mod finding_card;
mod job_card;
mod state;

pub use finding_card::{render_finding_card, severity_color};
pub use job_card::{render_job_card, job_status_color};
pub use state::{UnifiedBoardState, UnifiedBoardTab};

use crate::bugbounty::{Finding, FindingStatus, Severity};
use crate::job::JobManager;
use crate::{Job, JobStatus};
use egui::{Color32, RichText, Ui};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Job status columns for the board
const JOB_COLUMNS: &[(JobStatus, &str)] = &[
    (JobStatus::Pending, "Pending"),
    (JobStatus::Queued, "Queued"),
    (JobStatus::Running, "Running"),
    (JobStatus::Done, "Done"),
    (JobStatus::Failed, "Failed"),
];

/// Finding status columns for the board
const FINDING_COLUMNS: &[(FindingStatus, &str)] = &[
    (FindingStatus::Raw, "Raw"),
    (FindingStatus::NeedsRepro, "Needs Repro"),
    (FindingStatus::Verified, "Verified"),
    (FindingStatus::ReportDraft, "Report Draft"),
    (FindingStatus::Submitted, "Submitted"),
];

/// Terminal finding columns (collapsed)
const FINDING_TERMINAL: &[(FindingStatus, &str)] = &[
    (FindingStatus::Accepted, "Accepted"),
    (FindingStatus::Paid, "Paid"),
    (FindingStatus::FalsePositive, "FP"),
    (FindingStatus::Duplicate, "Dupe"),
];

/// Render the unified board view
pub fn render_unified_board(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    work_dir: &Path,
    cached_jobs: &[Job],
    job_manager: &Arc<Mutex<JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
) {
    // Check for job changes
    state.check_job_changes(job_manager);

    // Refresh data if needed
    if state.needs_refresh {
        state.refresh(job_manager);
    }

    // Top bar
    render_top_bar(ui, state);

    // Status message
    if let Some((ref msg, ok)) = state.status_message {
        ui.add_space(4.0);
        ui.label(
            RichText::new(msg)
                .small()
                .color(if ok {
                    Color32::from_rgb(34, 197, 94)
                } else {
                    Color32::from_rgb(220, 38, 38)
                }),
        );
    }

    ui.add_space(8.0);

    // Main content
    if state.selected_project.is_some() {
        match state.selected_tab {
            UnifiedBoardTab::Board => {
                render_board(ui, state, cached_jobs, job_manager);
            }
            UnifiedBoardTab::Dashboard => {
                render_dashboard(ui, state, cached_jobs);
            }
        }
    } else {
        render_project_list(ui, state);
    }

    // Detail panel
    render_detail_panel(ui, state, cached_jobs, work_dir, job_manager, group_manager, logs);

    // Dialogs
    render_fp_dialog(ui.ctx(), state);
    render_new_finding_dialog(ui.ctx(), state);

    // Handle drag end
    if ui.input(|i| i.pointer.any_released()) {
        state.dragged_job_id = None;
        state.dragged_finding_id = None;
    }
}

fn render_top_bar(ui: &mut Ui, state: &mut UnifiedBoardState) {
    ui.horizontal(|ui| {
        // Close button
        if ui.small_button("X").on_hover_text("Close Board").clicked() {
            state.close_requested = true;
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Unified Board").small().strong());

        ui.add_space(16.0);

        // Project selector
        if let Some(ref project_id) = state.selected_project {
            if ui.small_button(format!("Project: {}", project_id)).clicked() {
                state.select_project(None);
            }
        } else {
            ui.label(RichText::new("Select a project").small());
        }

        if state.selected_project.is_some() {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            ui.selectable_value(&mut state.selected_tab, UnifiedBoardTab::Board, RichText::new("Board").small());
            ui.selectable_value(&mut state.selected_tab, UnifiedBoardTab::Dashboard, RichText::new("Dashboard").small());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Refresh").clicked() {
                state.needs_refresh = true;
            }

            if state.selected_project.is_some() {
                if ui.small_button("+ New Finding").clicked() {
                    state.show_new_finding_dialog = true;
                }
            }
        });
    });
}

fn render_project_list(ui: &mut Ui, state: &mut UnifiedBoardState) {
    ui.heading("Projects");
    ui.add_space(8.0);

    if state.projects.is_empty() {
        ui.label("No projects found.");
        ui.label("Use 'kyco project discover' to scan for projects.");
        return;
    }

    let mut clicked_project_id: Option<String> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for project in state.projects.clone() {
            let findings_count = state
                .findings_by_project
                .get(&project.id)
                .map(|f| f.len())
                .unwrap_or(0);
            let findings_open = state
                .findings_by_project
                .get(&project.id)
                .map(|f| f.iter().filter(|f| !f.status.is_terminal()).count())
                .unwrap_or(0);
            let stats = state.project_stats.get(&project.id);
            let project_id = project.id.clone();

            ui.horizontal(|ui| {
                let response = ui.selectable_label(false, RichText::new(&project.id).strong());

                if response.clicked() {
                    clicked_project_id = Some(project_id.clone());
                }

                ui.label(format!(
                    "({}) - {} open / {} total findings",
                    project.platform.as_deref().unwrap_or("-"),
                    findings_open,
                    findings_count
                ));

                if let Some(s) = stats {
                    ui.add_space(12.0);
                    ui.label(format!("jobs: {} pending, {} running", s.jobs_pending, s.jobs_running));
                }
            });

            ui.add_space(4.0);
        }
    });

    if let Some(project_id) = clicked_project_id {
        state.select_project(Some(project_id));
    }
}

fn render_board(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    cached_jobs: &[Job],
    job_manager: &Arc<Mutex<JobManager>>,
) {
    let project_id = match &state.selected_project {
        Some(id) => id.clone(),
        None => return,
    };

    // Get data - clone to avoid borrow issues
    let jobs_by_status = state.get_jobs_by_status(cached_jobs);
    let findings: Vec<Finding> = state
        .findings_by_project
        .get(&project_id)
        .cloned()
        .unwrap_or_default();

    // Calculate heights upfront
    let total_height = ui.available_height();
    let jobs_height = (total_height * 0.30).max(150.0);
    let findings_height = (total_height * 0.60).max(200.0);

    // ========== JOBS PIPELINE ==========
    ui.horizontal(|ui| {
        ui.label(RichText::new("JOBS PIPELINE").small().strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("\u{2193} produces findings \u{2193}")
                .small()
                .color(Color32::from_rgb(100, 100, 120))
                .italics(),
        );
    });
    ui.add_space(4.0);

    egui::ScrollArea::horizontal()
        .id_salt("jobs_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(jobs_height);
                for (status, label) in JOB_COLUMNS {
                    let jobs: Vec<_> = jobs_by_status
                        .get(status)
                        .map(|v| v.iter().copied().collect())
                        .unwrap_or_default();

                    render_job_column(ui, state, label, &jobs, *status, job_manager, jobs_height);
                }
            });
        });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // ========== FINDINGS PIPELINE ==========
    ui.label(RichText::new("FINDINGS").small().strong());
    ui.add_space(4.0);

    egui::ScrollArea::horizontal()
        .id_salt("findings_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(findings_height);
                // Main columns
                for (status, label) in FINDING_COLUMNS {
                    let column_findings: Vec<_> = findings
                        .iter()
                        .filter(|f| &f.status == status)
                        .collect();

                    render_finding_column(ui, state, label, &column_findings, *status);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                // Terminal columns (mini)
                for (status, label) in FINDING_TERMINAL {
                    let column_findings: Vec<_> = findings
                        .iter()
                        .filter(|f| &f.status == status)
                        .collect();

                    render_mini_finding_column(ui, state, label, &column_findings);
                }
            });
        });
}

fn render_job_column(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    title: &str,
    jobs: &[&Job],
    status: JobStatus,
    job_manager: &Arc<Mutex<JobManager>>,
    column_height: f32,
) {
    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(250.0);
        ui.set_min_height(column_height);

        // Column header with status color dot
        ui.horizontal(|ui| {
            let color = job_status_color(status);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.0, color);

            ui.label(RichText::new(title).small().strong());
            ui.label(RichText::new(format!("({})", jobs.len())).small().color(Color32::GRAY));
        });

        ui.add_space(4.0);

        // Cards area - takes remaining space
        let cards_height = (column_height - 40.0).max(100.0);
        let response = egui::ScrollArea::vertical()
            .max_height(cards_height)
            .id_salt(format!("job_col_{:?}", status))
            .show(ui, |ui| {
                ui.set_min_height(cards_height - 10.0);
                ui.set_min_width(190.0);

                for job in jobs {
                    render_job_card(ui, state, job);
                    ui.add_space(4.0);
                }

                if jobs.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Drop jobs here")
                                .color(Color32::from_rgb(80, 80, 90))
                                .italics(),
                        );
                    });
                }
            });

        // Handle drag-and-drop for jobs
        if let Some(dragged_id) = state.dragged_job_id {
            if response.inner_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default()) {
                if ui.input(|i| i.pointer.any_released()) {
                    match status {
                        JobStatus::Queued => {
                            state.queue_job(dragged_id, job_manager);
                        }
                        JobStatus::Failed => {
                            state.kill_job(dragged_id, job_manager);
                        }
                        _ => {}
                    }
                    state.dragged_job_id = None;
                }
            }
        }
    });
}

fn render_finding_column(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    title: &str,
    findings: &[&Finding],
    status: FindingStatus,
) {
    // Get column height BEFORE vertical layout
    let column_height = ui.available_height();

    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(250.0);
        ui.set_min_height(column_height);

        // Column header (sticky-like)
        ui.horizontal(|ui| {
            // Status color dot
            let color = match status {
                FindingStatus::Raw => Color32::from_rgb(156, 163, 175),
                FindingStatus::NeedsRepro => Color32::from_rgb(251, 191, 36),
                FindingStatus::Verified => Color32::from_rgb(34, 197, 94),
                FindingStatus::ReportDraft => Color32::from_rgb(59, 130, 246),
                FindingStatus::Submitted => Color32::from_rgb(168, 85, 247),
                _ => Color32::GRAY,
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.0, color);

            ui.label(RichText::new(title).small().strong());
            ui.label(RichText::new(format!("({})", findings.len())).small().color(Color32::GRAY));
        });

        ui.add_space(4.0);

        // Cards area - takes remaining space
        let cards_height = (column_height - 40.0).max(100.0);
        let response = egui::ScrollArea::vertical()
            .max_height(cards_height)
            .id_salt(format!("finding_col_{:?}", status))
            .show(ui, |ui| {
                ui.set_min_height(cards_height - 10.0);
                ui.set_min_width(190.0);

                for finding in findings {
                    render_finding_card(ui, state, finding);
                    ui.add_space(4.0);
                }

                if findings.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Drop findings here")
                                .color(Color32::from_rgb(80, 80, 90))
                                .italics(),
                        );
                    });
                }
            });

        // Handle drag-and-drop for findings
        if let Some(ref dragged_id) = state.dragged_finding_id.clone() {
            if response.inner_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default()) {
                if ui.input(|i| i.pointer.any_released()) {
                    state.move_finding(dragged_id, status);
                    state.dragged_finding_id = None;
                }
            }
        }
    });
}

fn render_mini_finding_column(
    ui: &mut Ui,
    _state: &mut UnifiedBoardState,
    title: &str,
    findings: &[&Finding],
) {
    ui.vertical(|ui| {
        ui.set_min_width(80.0);
        ui.set_max_width(100.0);

        ui.label(RichText::new(title).small());
        ui.label(RichText::new(format!("{}", findings.len())).strong());

        for finding in findings.iter().take(3) {
            ui.label(
                RichText::new(&finding.id)
                    .small()
                    .color(severity_color(finding.severity)),
            );
        }

        if findings.len() > 3 {
            ui.label(RichText::new(format!("+{} more", findings.len() - 3)).small());
        }
    });
}

fn render_dashboard(ui: &mut Ui, state: &mut UnifiedBoardState, cached_jobs: &[Job]) {
    let project_id = match &state.selected_project {
        Some(id) => id.clone(),
        None => return,
    };

    let findings = state.get_project_findings();
    let jobs = state.get_project_jobs(cached_jobs);

    ui.heading(format!("Dashboard: {}", project_id));
    ui.add_space(8.0);

    // Job stats
    let jobs_pending = jobs.iter().filter(|j| j.status == JobStatus::Pending).count();
    let jobs_queued = jobs.iter().filter(|j| j.status == JobStatus::Queued).count();
    let jobs_running = jobs.iter().filter(|j| j.status == JobStatus::Running).count();
    let jobs_done = jobs.iter().filter(|j| j.status == JobStatus::Done).count();
    let jobs_failed = jobs.iter().filter(|j| j.status == JobStatus::Failed).count();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Jobs").strong());
        ui.label(format!(
            "pending {} | queued {} | running {} | done {} | failed {} (total {})",
            jobs_pending, jobs_queued, jobs_running, jobs_done, jobs_failed, jobs.len()
        ));
    });

    // Finding stats
    let findings_raw = findings.iter().filter(|f| f.status == FindingStatus::Raw).count();
    let findings_verified = findings.iter().filter(|f| f.status == FindingStatus::Verified).count();
    let findings_submitted = findings.iter().filter(|f| f.status == FindingStatus::Submitted).count();
    let findings_terminal = findings.iter().filter(|f| f.status.is_terminal()).count();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Findings").strong());
        ui.label(format!(
            "raw {} | verified {} | submitted {} | terminal {} (total {})",
            findings_raw, findings_verified, findings_submitted, findings_terminal, findings.len()
        ));
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // By severity
    let mut sev_crit = 0usize;
    let mut sev_high = 0usize;
    let mut sev_med = 0usize;
    let mut sev_low = 0usize;
    let mut sev_info = 0usize;
    for f in &findings {
        match f.severity {
            Some(Severity::Critical) => sev_crit += 1,
            Some(Severity::High) => sev_high += 1,
            Some(Severity::Medium) => sev_med += 1,
            Some(Severity::Low) => sev_low += 1,
            Some(Severity::Info) => sev_info += 1,
            None => {}
        }
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("By severity").strong());
        ui.label(format!(
            "crit {} | high {} | med {} | low {} | info {}",
            sev_crit, sev_high, sev_med, sev_low, sev_info,
        ));
    });
}

fn render_detail_panel(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    cached_jobs: &[Job],
    work_dir: &Path,
    job_manager: &Arc<Mutex<JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
) {
    // Job detail
    if let Some(job_id) = state.selected_job {
        let job = cached_jobs.iter().find(|j| j.id == job_id);
        if let Some(job) = job {
            render_job_detail(ui, state, job, job_manager);
        } else {
            state.selected_job = None;
        }
        return;
    }

    // Finding detail
    if let Some(ref finding_id) = state.selected_finding.clone() {
        let finding = state
            .findings_by_project
            .values()
            .flatten()
            .find(|f| &f.id == finding_id)
            .cloned();

        if let Some(finding) = finding {
            render_finding_detail(ui, state, &finding, work_dir, job_manager, group_manager, logs);
        } else {
            state.selected_finding = None;
        }
    }
}

fn render_job_detail(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    job: &Job,
    job_manager: &Arc<Mutex<JobManager>>,
) {
    egui::SidePanel::right("job_detail")
        .default_width(350.0)
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("Job #{}", job.id));
                if ui.button("X").clicked() {
                    state.selected_job = None;
                }
            });

            ui.add_space(8.0);

            // Status
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(
                    RichText::new(format!("{}", job.status))
                        .color(job_status_color(job.status)),
                );
            });

            // Skill
            ui.horizontal(|ui| {
                ui.label("Skill:");
                ui.label(&job.skill);
            });

            // Agent
            ui.horizontal(|ui| {
                ui.label("Agent:");
                ui.label(&job.agent_id);
            });

            // Target
            if !job.target.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Target:");
                    ui.label(&job.target);
                });
            }

            ui.add_space(8.0);

            // Actions
            ui.horizontal(|ui| {
                match job.status {
                    JobStatus::Pending => {
                        if ui.button("Queue").clicked() {
                            state.queue_job(job.id, job_manager);
                        }
                    }
                    JobStatus::Queued | JobStatus::Running => {
                        if ui.button("Kill").clicked() {
                            state.kill_job(job.id, job_manager);
                        }
                    }
                    _ => {}
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Linked findings
            ui.label(RichText::new("Linked Findings").strong());
            if job.bugbounty_finding_ids.is_empty() {
                ui.label(RichText::new("None").italics().color(Color32::GRAY));
            } else {
                for fid in &job.bugbounty_finding_ids {
                    if ui.link(fid).clicked() {
                        state.selected_finding = Some(fid.clone());
                        state.selected_job = None;
                    }
                }
            }

            // Result
            if let Some(ref result) = job.result {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Result").strong());

                if let Some(ref state_str) = result.state {
                    ui.label(format!("State: {}", state_str));
                }
                if let Some(ref title) = result.title {
                    ui.label(format!("Title: {}", title));
                }
                if let Some(ref summary) = result.summary {
                    ui.label(RichText::new("Summary:").small());
                    ui.label(summary);
                }
            }

            // Error
            if let Some(ref err) = job.error_message {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Error: {}", err))
                        .color(Color32::from_rgb(239, 68, 68)),
                );
            }
        });
}

fn render_finding_detail(
    ui: &mut Ui,
    state: &mut UnifiedBoardState,
    finding: &Finding,
    work_dir: &Path,
    job_manager: &Arc<Mutex<JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
) {
    egui::SidePanel::right("finding_detail")
        .default_width(400.0)
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&finding.id);
                if ui.button("X").clicked() {
                    state.selected_finding = None;
                }
            });

            ui.add_space(8.0);

            // Quick actions
            ui.horizontal(|ui| {
                if ui.button("Start Verify Job").clicked() {
                    start_finding_job(state, work_dir, job_manager, group_manager, logs, finding, "verify");
                }
                if ui.button("Mark FP").clicked() {
                    state.show_fp_dialog = true;
                    state.fp_target_finding_id = Some(finding.id.clone());
                }
            });

            ui.add_space(8.0);

            // Status
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.label(finding.status.as_str());
            });

            // Severity
            ui.horizontal(|ui| {
                ui.label("Severity:");
                let sev_text = finding
                    .severity
                    .map(|s| s.as_str().to_uppercase())
                    .unwrap_or_else(|| "-".to_string());
                ui.label(RichText::new(sev_text).color(severity_color(finding.severity)));
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.heading(&finding.title);

            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(ref scenario) = finding.attack_scenario {
                    ui.label(RichText::new("Attack Scenario").strong());
                    ui.label(scenario);
                    ui.add_space(8.0);
                }

                if let Some(ref impact) = finding.impact {
                    ui.label(RichText::new("Impact").strong());
                    ui.label(impact);
                    ui.add_space(8.0);
                }

                if !finding.affected_assets.is_empty() {
                    ui.label(RichText::new("Affected Assets").strong());
                    for asset in &finding.affected_assets {
                        ui.label(format!("- {}", asset));
                    }
                    ui.add_space(8.0);
                }

                // Linked Jobs
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Linked Jobs").strong());

                let should_load = match &state.cached_linked_jobs {
                    Some((cached_for, _)) => cached_for != &finding.id,
                    None => true,
                };
                if should_load {
                    state.load_linked_jobs(&finding.id);
                }

                match &state.cached_linked_jobs {
                    Some((cached_for, jobs)) if cached_for == &finding.id && !jobs.is_empty() => {
                        for job in jobs.iter().take(10) {
                            let job_label = job
                                .kyco_job_id
                                .map(|id| format!("#{}", id))
                                .unwrap_or_else(|| job.id.clone());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(job_label).monospace());
                                ui.label(format!("[{}]", job.status));
                                if let Some(ref mode) = job.mode {
                                    ui.label(mode);
                                }
                            });
                        }
                    }
                    Some((cached_for, _)) if cached_for == &finding.id => {
                        ui.label(RichText::new("No jobs linked").italics().color(Color32::GRAY));
                    }
                    _ => {}
                }
            });
        });
}

fn start_finding_job(
    state: &mut UnifiedBoardState,
    work_dir: &Path,
    job_manager: &Arc<Mutex<JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
    finding: &Finding,
    skill: &str,
) {
    let Some(project_id) = state.selected_project.clone() else {
        state.status_message = Some(("No project selected".to_string(), false));
        return;
    };

    let Some(project) = state.projects.iter().find(|p| p.id == project_id).cloned() else {
        state.status_message = Some((format!("Project not found: {}", project_id), false));
        return;
    };

    let project_root = std::path::PathBuf::from(&project.root_path);
    let project_root_abs = if project_root.is_absolute() {
        project_root
    } else {
        work_dir.join(project_root)
    };

    let selection = super::selection::SelectionContext {
        app_name: Some("UnifiedBoard".to_string()),
        file_path: None,
        selected_text: None,
        line_number: Some(1),
        line_end: None,
        workspace_path: Some(project_root_abs),
        ..Default::default()
    };

    let prompt = format!(
        "Verify the finding {} ({}). Output: next_context.findings[] with updates.",
        finding.id, finding.title
    );

    let agents = vec!["claude".to_string()];
    let created = super::jobs::create_jobs_from_selection_multi(
        job_manager,
        group_manager,
        &selection,
        &agents,
        skill,
        &prompt,
        logs,
        false,
    );

    let Some(created) = created else {
        state.status_message = Some(("Failed to create job".to_string(), false));
        return;
    };

    if let Ok(mut manager) = job_manager.lock() {
        for job_id in &created.job_ids {
            if let Some(job) = manager.get_mut(*job_id) {
                job.bugbounty_project_id = Some(project_id.to_string());
                job.bugbounty_finding_ids = vec![finding.id.clone()];
            }
        }
        manager.touch();
    }

    for job_id in &created.job_ids {
        super::jobs::queue_job(job_manager, *job_id, logs);
    }

    state.status_message = Some((format!("Created job #{}", created.job_ids[0]), true));
}

fn render_fp_dialog(ctx: &egui::Context, state: &mut UnifiedBoardState) {
    if !state.show_fp_dialog {
        return;
    }

    egui::Window::new("Mark False Positive")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let target = state.fp_target_finding_id.clone().unwrap_or_default();
            ui.label(format!("Finding: {}", target));
            ui.add_space(8.0);
            ui.label("Reason:");
            ui.text_edit_multiline(&mut state.fp_reason_input);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    state.show_fp_dialog = false;
                    state.fp_target_finding_id = None;
                }
                if ui.button("Confirm").clicked() {
                    let reason = state.fp_reason_input.trim().to_string();
                    if !target.is_empty() && !reason.is_empty() {
                        state.mark_false_positive(&target, &reason);
                        state.needs_refresh = true;
                    }
                    state.show_fp_dialog = false;
                    state.fp_target_finding_id = None;
                }
            });
        });
}

fn render_new_finding_dialog(ctx: &egui::Context, state: &mut UnifiedBoardState) {
    if !state.show_new_finding_dialog {
        return;
    }

    egui::Window::new("New Finding")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Title:");
            ui.text_edit_singleline(&mut state.new_finding_title);
            ui.add_space(8.0);

            ui.label("Severity:");
            egui::ComboBox::from_id_salt("new_finding_severity")
                .selected_text(state.new_finding_severity.clone())
                .show_ui(ui, |ui| {
                    for sev in ["critical", "high", "medium", "low", "info"] {
                        ui.selectable_value(&mut state.new_finding_severity, sev.to_string(), sev);
                    }
                });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    state.show_new_finding_dialog = false;
                    state.new_finding_title.clear();
                }
                if ui.button("Create").clicked() {
                    let title = state.new_finding_title.trim().to_string();
                    if !title.is_empty() {
                        let sev = state.new_finding_severity.clone();
                        state.create_finding(&title, &sev);
                    }
                }
            });
        });
}
