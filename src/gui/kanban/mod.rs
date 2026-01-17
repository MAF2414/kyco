//! Kanban board view for BugBounty findings
//!
//! Displays findings in a Kanban-style board with columns for each status.

mod card;
mod column;
mod flow_graph;
mod state;

pub use flow_graph::{render_flow_graph, render_flow_summary};
pub use state::{KanbanState, KanbanTab};

use crate::bugbounty::{Finding, FindingStatus, Severity};
use egui::{Color32, RichText, Ui};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Render the Kanban board view
pub fn render_kanban_view(
    ui: &mut Ui,
    state: &mut KanbanState,
    work_dir: &Path,
    cached_jobs: &[crate::Job],
    job_manager: &Arc<Mutex<crate::job::JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
) {
    // Refresh data if needed
    if state.needs_refresh {
        state.refresh();
    }

    // Top bar: project selector + actions
    render_top_bar(ui, state);

    if let Some((ref msg, ok)) = state.status_message {
        ui.add_space(6.0);
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

    // Main content: Kanban columns or project list
    if state.selected_project.is_some() {
        match state.selected_tab {
            KanbanTab::Dashboard => render_dashboard(ui, state),
            KanbanTab::Board => render_kanban_board(ui, state),
            KanbanTab::Jobs => render_jobs_view(ui, state, cached_jobs),
        }
    } else {
        render_project_list(ui, state);
    }

    // Finding detail panel (slide-in from right)
    if state.selected_finding.is_some() {
        render_finding_detail(ui, state, work_dir, job_manager, group_manager, logs);
    }

    render_mark_fp_dialog(ui.ctx(), state);
    render_new_finding_dialog(ui.ctx(), state);
}

fn render_top_bar(ui: &mut Ui, state: &mut KanbanState) {
    ui.horizontal(|ui| {
        ui.heading("BugBounty");

        ui.add_space(20.0);

        // Project selector
        if let Some(ref project_id) = state.selected_project {
            if ui.button(format!("Project: {}", project_id)).clicked() {
                state.select_project(None);
            }
        } else {
            ui.label("Select a project");
        }

        if state.selected_project.is_some() {
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            ui.selectable_value(&mut state.selected_tab, KanbanTab::Dashboard, "Dashboard");
            ui.selectable_value(&mut state.selected_tab, KanbanTab::Board, "Kanban");
            ui.selectable_value(&mut state.selected_tab, KanbanTab::Jobs, "Jobs");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh").clicked() {
                state.needs_refresh = true;
            }

            if state.selected_project.is_some() {
                if ui.button("+ New Finding").clicked() {
                    state.show_new_finding_dialog = true;
                }
            }
        });
    });
}

fn render_project_list(ui: &mut Ui, state: &mut KanbanState) {
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
            let stats = state
                .project_stats
                .get(&project.id)
                .map(|s| (s.jobs_pending, s.jobs_running, s.last_activity_at));
            let project_id = project.id.clone();

            ui.horizontal(|ui| {
                let response = ui.selectable_label(
                    false,
                    RichText::new(&project.id).strong(),
                );

                if response.clicked() {
                    clicked_project_id = Some(project_id.clone());
                }

                ui.label(format!(
                    "({}) - {} open / {} total findings",
                    project.platform.as_deref().unwrap_or("-"),
                    findings_open,
                    findings_count
                ));

                if let Some((jobs_pending, jobs_running, last_activity_at)) = stats {
                    ui.add_space(12.0);
                    ui.label(format!(
                        "jobs: {} pending, {} running",
                        jobs_pending, jobs_running
                    ));
                    ui.add_space(12.0);
                    ui.label(format!("last: {}", format_timestamp(last_activity_at)));
                }
            });

            ui.add_space(4.0);
        }
    });

    if let Some(project_id) = clicked_project_id {
        state.select_project(Some(project_id));
    }
}

fn render_kanban_board(ui: &mut Ui, state: &mut KanbanState) {
    let project_id = match &state.selected_project {
        Some(id) => id.clone(),
        None => return,
    };

    let findings = match state.findings_by_project.get(&project_id) {
        Some(f) => f.clone(),
        None => Vec::new(),
    };

    // Define visible columns
    let columns = [
        (FindingStatus::Raw, "Raw"),
        (FindingStatus::NeedsRepro, "Needs Repro"),
        (FindingStatus::Verified, "Verified"),
        (FindingStatus::ReportDraft, "Report Draft"),
        (FindingStatus::Submitted, "Submitted"),
    ];

    // Terminal columns (collapsed)
    let terminal_columns = [
        (FindingStatus::Accepted, "Accepted"),
        (FindingStatus::Paid, "Paid"),
        (FindingStatus::FalsePositive, "FP"),
        (FindingStatus::Duplicate, "Dupe"),
    ];

    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            // Main columns
            for (status, label) in &columns {
                let column_findings: Vec<_> = findings
                    .iter()
                    .filter(|f| &f.status == status)
                    .collect();

                render_column(ui, state, label, &column_findings, *status);
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(20.0);

            // Terminal columns (smaller)
            for (status, label) in &terminal_columns {
                let column_findings: Vec<_> = findings
                    .iter()
                    .filter(|f| &f.status == status)
                    .collect();

                render_mini_column(ui, state, label, &column_findings);
            }
        });
    });
}

fn render_dashboard(ui: &mut Ui, state: &mut KanbanState) {
    let project_id = match &state.selected_project {
        Some(id) => id.clone(),
        None => return,
    };

    let findings = state
        .findings_by_project
        .get(&project_id)
        .cloned()
        .unwrap_or_default();

    ui.heading(format!("Dashboard: {}", project_id));
    ui.add_space(8.0);

    if let Some(stats) = state.project_stats.get(&project_id) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Jobs").strong());
            ui.label(format!(
                "pending {} | running {} | done {} | failed {} (total {})",
                stats.jobs_pending, stats.jobs_running, stats.jobs_done, stats.jobs_failed, stats.jobs_total
            ));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Findings").strong());
            ui.label(format!(
                "raw {} | verified {} | submitted+ {} | terminal {} (total {})",
                stats.findings_raw,
                stats.findings_verified,
                stats.findings_submitted,
                stats.findings_terminal,
                stats.findings_total
            ));
        });
    } else {
        ui.label(RichText::new("Project stats unavailable").italics().color(Color32::GRAY));
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Findings breakdown by severity
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
            sev_crit, sev_high, sev_med, sev_low, sev_info
        ));
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Recent activity
    ui.label(RichText::new("Recent activity").strong());

    let should_load = match &state.cached_recent_jobs {
        Some((cached_for, _)) => cached_for != &project_id,
        None => true,
    };
    if should_load {
        state.load_recent_jobs(&project_id, 10);
    }

    if let Some((cached_for, jobs)) = &state.cached_recent_jobs {
        if cached_for == &project_id && !jobs.is_empty() {
            for job in jobs.iter().take(10) {
                let id_display = job
                    .kyco_job_id
                    .map(|id| format!("#{}", id))
                    .unwrap_or_else(|| job.id.clone());
                let mode = job.mode.as_deref().unwrap_or("-");
                let result_state = job.result_state.as_deref().unwrap_or("-");
                let when = format_timestamp(job.completed_at.or(job.started_at).or(Some(job.created_at)));
                ui.label(
                    RichText::new(format!("{} [{}] {} ({}) - {}", id_display, job.status, mode, result_state, when))
                        .monospace()
                        .color(Color32::GRAY),
                );
            }
        }
    }

    let mut recent_findings = findings.clone();
    recent_findings.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    if !recent_findings.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("Recent findings").strong());
        for f in recent_findings.iter().take(10) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&f.id).monospace());
                ui.label(RichText::new(f.status.as_str()).small().color(Color32::GRAY));
                ui.label(RichText::new(&f.title).small());
            });
        }
    }
}

fn render_jobs_view(ui: &mut Ui, state: &mut KanbanState, cached_jobs: &[crate::Job]) {
    let project_id = match &state.selected_project {
        Some(id) => id.clone(),
        None => return,
    };

    ui.heading("Jobs");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Agent:");
        let mut agents: Vec<String> = cached_jobs
            .iter()
            .filter(|j| j.bugbounty_project_id.as_deref() == Some(project_id.as_str()))
            .map(|j| j.agent_id.clone())
            .collect();
        agents.sort();
        agents.dedup();
        let selected_label = state
            .jobs_filter_agent
            .as_deref()
            .unwrap_or("all");
        egui::ComboBox::from_id_salt("bb_jobs_agent")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.jobs_filter_agent.is_none(), "all")
                    .clicked()
                {
                    state.jobs_filter_agent = None;
                }
                for agent in &agents {
                    if ui
                        .selectable_label(
                            state.jobs_filter_agent.as_deref() == Some(agent.as_str()),
                            agent,
                        )
                        .clicked()
                    {
                        state.jobs_filter_agent = Some(agent.clone());
                    }
                }
            });

        ui.add_space(12.0);
        ui.label("State:");
        ui.text_edit_singleline(&mut state.jobs_filter_state);
        ui.add_space(12.0);
        ui.label("File:");
        ui.text_edit_singleline(&mut state.jobs_filter_file);
        ui.add_space(12.0);
        ui.label("Finding:");
        ui.text_edit_singleline(&mut state.jobs_filter_finding);
    });

    let state_q = state.jobs_filter_state.trim().to_string();
    let file_q = state.jobs_filter_file.trim().to_string();
    let finding_q = state.jobs_filter_finding.trim().to_string();

    let mut jobs: Vec<&crate::Job> = cached_jobs
        .iter()
        .filter(|j| j.bugbounty_project_id.as_deref() == Some(project_id.as_str()))
        .collect();

    if let Some(ref agent) = state.jobs_filter_agent {
        jobs.retain(|j| &j.agent_id == agent);
    }
    if !state_q.is_empty() {
        jobs.retain(|j| {
            j.result
                .as_ref()
                .and_then(|r| r.state.as_deref())
                .map(|s| s.contains(&state_q))
                .unwrap_or(false)
        });
    }
    if !file_q.is_empty() {
        jobs.retain(|j| {
            j.source_file.to_string_lossy().contains(&file_q) || j.target.contains(&file_q)
        });
    }
    if !finding_q.is_empty() {
        jobs.retain(|j| j.bugbounty_finding_ids.iter().any(|f| f.contains(&finding_q)));
    }

    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    if jobs.is_empty() {
        ui.label(RichText::new("No jobs match the current filters.").italics().color(Color32::GRAY));
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for job in jobs {
            let state_label = job
                .result
                .as_ref()
                .and_then(|r| r.state.as_deref())
                .unwrap_or("-");
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("#{}", job.id)).monospace());
                ui.label(format!("[{}]", job.status));
                ui.label(&job.skill);
                ui.label(RichText::new(&job.agent_id).small().color(Color32::GRAY));
                ui.label(RichText::new(state_label).small().color(Color32::GRAY));
                ui.label(RichText::new(job.target.clone()).small());
            });
            ui.add_space(4.0);
        }
    });
}

fn render_column(
    ui: &mut Ui,
    state: &mut KanbanState,
    title: &str,
    findings: &[&Finding],
    status: FindingStatus,
) {
    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(250.0);

        // Column header
        ui.horizontal(|ui| {
            ui.heading(title);
            ui.label(format!("({})", findings.len()));
        });

        ui.add_space(4.0);

        // Drop zone
        let response = egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 50.0)
            .show(ui, |ui| {
                ui.set_min_height(200.0);

                for finding in findings {
                    card::render_finding_card(ui, state, finding);
                    ui.add_space(4.0);
                }

                // Drop zone indicator
                if findings.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Drop findings here")
                                .color(Color32::GRAY)
                                .italics(),
                        );
                    });
                }
            });

        // Handle drag-and-drop
        let dragged_id = state.dragged_finding_id.clone();
        if let Some(dragged_id) = dragged_id {
            if response.inner_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default()) {
                if ui.input(|i| i.pointer.any_released()) {
                    // Move finding to this column
                    state.move_finding(&dragged_id, status);
                    state.dragged_finding_id = None;
                }
            }
        }
    });
}

fn render_mini_column(
    ui: &mut Ui,
    _state: &mut KanbanState,
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

fn render_finding_detail(
    ui: &mut Ui,
    state: &mut KanbanState,
    work_dir: &Path,
    job_manager: &Arc<Mutex<crate::job::JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
) {
    let finding_id = match &state.selected_finding {
        Some(id) => id.clone(),
        None => return,
    };

    // Find the finding
    let finding = state
        .findings_by_project
        .values()
        .flatten()
        .find(|f| f.id == finding_id)
        .cloned();

    let finding = match finding {
        Some(f) => f,
        None => {
            state.selected_finding = None;
            return;
        }
    };

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
            ui.horizontal(|ui| {
                if ui.button("Start Verify Job").clicked() {
                    if let Some(project_id) = state.selected_project.clone() {
                        start_finding_job(
                            state,
                            work_dir,
                            job_manager,
                            group_manager,
                            logs,
                            &project_id,
                            &finding,
                            "flow-trace",
                            format!(
                                "Verify the following finding and produce concrete evidence.\n\n- Finding: {} ({})\n- Goal: confirm exploitability OR explain why it's a false positive.\n- Output: include next_context.findings[] for this finding; add artifacts/flow_edges if relevant.\n",
                                finding.id, finding.title
                            ),
                        );
                    }
                }
                if ui.button("Start Flow Trace").clicked() {
                    if let Some(project_id) = state.selected_project.clone() {
                        start_finding_job(
                            state,
                            work_dir,
                            job_manager,
                            group_manager,
                            logs,
                            &project_id,
                            &finding,
                            "flow-trace",
                            format!(
                                "Create a cross-file flow trace for this finding.\n\n- Finding: {} ({})\n- Output: include next_context.flow_edges[] (and finding updates if needed).\n",
                                finding.id, finding.title
                            ),
                        );
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Export Report (Markdown)").clicked() {
                    state.export_finding(&finding.id, "markdown");
                }
                if ui.button("Mark FP").clicked() {
                    state.show_fp_dialog = true;
                    state.fp_target_finding_id = Some(finding.id.clone());
                    state.fp_reason_input = finding
                        .fp_reason
                        .clone()
                        .unwrap_or_else(|| "false positive".to_string());
                }
            });

            ui.add_space(8.0);

            // Status selector
            ui.horizontal(|ui| {
                ui.label("Status:");
                egui::ComboBox::from_id_salt("status_select")
                    .selected_text(finding.status.as_str())
                    .show_ui(ui, |ui| {
                        for status in &[
                            FindingStatus::Raw,
                            FindingStatus::NeedsRepro,
                            FindingStatus::Verified,
                            FindingStatus::ReportDraft,
                            FindingStatus::Submitted,
                            FindingStatus::Triaged,
                            FindingStatus::Accepted,
                            FindingStatus::Paid,
                            FindingStatus::FalsePositive,
                            FindingStatus::Duplicate,
                        ] {
                            if ui.selectable_label(finding.status == *status, status.as_str()).clicked() {
                                state.move_finding(&finding.id, *status);
                            }
                        }
                    });
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

                if let Some(ref preconditions) = finding.preconditions {
                    ui.label(RichText::new("Preconditions").strong());
                    ui.label(preconditions);
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

                if let Some(ref cwe) = finding.cwe_id {
                    ui.label(format!("CWE: {}", cwe));
                }

                if let Some(ref taint) = finding.taint_path {
                    ui.label(RichText::new("Taint Path").strong());
                    ui.code(taint);
                    ui.add_space(8.0);
                }

                // Linked Jobs section
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Linked Jobs").strong());

                let should_load = match &state.cached_linked_jobs {
                    Some((cached_for, _)) => cached_for != &finding_id,
                    None => true,
                };
                if should_load {
                    state.load_linked_jobs(&finding_id);
                }

                match &state.cached_linked_jobs {
                    Some((cached_for, jobs)) if cached_for == &finding_id && !jobs.is_empty() => {
                        for job in jobs.iter().take(10) {
                            let job_id_label = job
                                .kyco_job_id
                                .map(|id| format!("#{}", id))
                                .unwrap_or_else(|| job.id.clone());
                            let mode = job.mode.as_deref().unwrap_or("-");
                            let result_state = job.result_state.as_deref().unwrap_or("-");
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(job_id_label).monospace());
                                ui.label(format!("[{}]", job.status));
                                ui.label(mode);
                                ui.label(RichText::new(result_state).small().color(Color32::GRAY));
                            });
                        }
                    }
                    Some((cached_for, _)) if cached_for == &finding_id => {
                        ui.label(RichText::new("No jobs linked").italics().color(Color32::GRAY));
                    }
                    _ => {}
                }

                // Flow Graph section
                ui.separator();
                ui.add_space(8.0);

                // Load flow trace if not cached or if it's for a different finding
                let should_load = match &state.cached_flow_trace {
                    Some(trace) => trace.finding_id != finding_id,
                    None => true,
                };
                if should_load {
                    state.load_flow_trace(&finding_id);
                }

                if let Some(ref trace) = state.cached_flow_trace {
                    if trace.finding_id == finding_id {
                        flow_graph::render_flow_graph(ui, trace);
                    }
                }
            });
        });
}

fn render_mark_fp_dialog(ctx: &egui::Context, state: &mut KanbanState) {
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

fn render_new_finding_dialog(ctx: &egui::Context, state: &mut KanbanState) {
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

fn start_finding_job(
    state: &mut KanbanState,
    work_dir: &Path,
    job_manager: &Arc<Mutex<crate::job::JobManager>>,
    group_manager: &Arc<Mutex<crate::job::GroupManager>>,
    logs: &mut Vec<crate::LogEvent>,
    project_id: &str,
    finding: &Finding,
    skill: &str,
    prompt: String,
) {
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
        app_name: Some("Kanban".to_string()),
        file_path: None,
        selected_text: None,
        line_number: Some(1),
        line_end: None,
        workspace_path: Some(project_root_abs),
        ..Default::default()
    };

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

    if created.job_ids.len() == 1 {
        state.status_message = Some((format!("Created job #{}", created.job_ids[0]), true));
    } else {
        state.status_message = Some((
            format!(
                "Created jobs: {}",
                created
                    .job_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            true,
        ));
    }
    state.selected_tab = KanbanTab::Jobs;
}

/// Get color for severity level
pub fn severity_color(severity: Option<Severity>) -> Color32 {
    match severity {
        Some(Severity::Critical) => Color32::from_rgb(220, 38, 38),   // Red
        Some(Severity::High) => Color32::from_rgb(249, 115, 22),      // Orange
        Some(Severity::Medium) => Color32::from_rgb(234, 179, 8),     // Yellow
        Some(Severity::Low) => Color32::from_rgb(34, 197, 94),        // Green
        Some(Severity::Info) => Color32::from_rgb(59, 130, 246),      // Blue
        None => Color32::GRAY,
    }
}

fn format_timestamp(ts_millis: Option<i64>) -> String {
    ts_millis
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}
