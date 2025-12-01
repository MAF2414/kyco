//! Job management functionality for the GUI
//!
//! This module contains all job-related logic including:
//! - Job creation and management
//! - Job list rendering
//! - Job file I/O operations

use super::app::{
    SelectionContext, BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use super::detail_panel::status_color;
use crate::job::JobManager;
use crate::{CommentTag, Job, JobId, JobStatus, LogEvent, Target};
use eframe::egui::{self, Color32, RichText, ScrollArea};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Refresh cached jobs from JobManager
pub fn refresh_jobs(job_manager: &Arc<Mutex<JobManager>>) -> Vec<Job> {
    if let Ok(manager) = job_manager.lock() {
        manager.jobs().into_iter().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Create a job from the selection popup
pub fn create_job_from_selection(
    job_manager: &Arc<Mutex<JobManager>>,
    selection: &SelectionContext,
    agent: &str,
    mode: &str,
    prompt: &str,
    logs: &mut Vec<LogEvent>,
) -> Option<JobId> {
    let file_path = selection.file_path.clone()?;
    let line_number = selection.line_number.unwrap_or(1);
    let line_end = selection.line_end;

    let tag = CommentTag {
        file_path: PathBuf::from(&file_path),
        line_number,
        raw_line: format!("// @{}:{} {}", agent, mode, prompt),
        agent: agent.to_string(),
        mode: mode.to_string(),
        target: Target::Block,
        status_marker: None,
        description: if prompt.is_empty() {
            None
        } else {
            Some(prompt.to_string())
        },
        job_id: None,
    };

    if let Ok(mut manager) = job_manager.lock() {
        match manager.create_job_with_range(&tag, agent, line_end) {
            Ok(job_id) => {
                logs.push(LogEvent::system(format!("Created job #{}", job_id)));
                return Some(job_id);
            }
            Err(e) => {
                logs.push(LogEvent::error(format!("Failed to create job: {}", e)));
            }
        }
    }
    None
}

/// Queue a job for execution
pub fn queue_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Queued);
        logs.push(LogEvent::system(format!("Queued job #{}", job_id)));
    }
}

/// Apply job changes (merge worktree to main)
pub fn apply_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Merged);
        logs.push(LogEvent::system(format!("Applied job #{}", job_id)));
    }
}

/// Reject job changes
pub fn reject_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Rejected);
        logs.push(LogEvent::system(format!("Rejected job #{}", job_id)));
    }
}

/// Write a job request file to the gui_jobs directory
#[allow(dead_code)]
pub fn write_job_request(
    work_dir: &PathBuf,
    selection: &SelectionContext,
    agent: &str,
    mode: &str,
    prompt: &str,
) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;

    let kyco_dir = work_dir.join(".kyco");
    let jobs_dir = kyco_dir.join("gui_jobs");
    fs::create_dir_all(&jobs_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let job_file = jobs_dir.join(format!("job_{}.json", timestamp));

    let (line_start, line_end, line_count) = if let Some(text) = &selection.selected_text {
        let lines: Vec<&str> = text.lines().collect();
        let count = lines.len();
        let start = selection.line_number.unwrap_or(1);
        let end = start + count.saturating_sub(1);
        (Some(start), Some(end), Some(count))
    } else {
        (None, None, None)
    };

    let job = serde_json::json!({
        "agent": agent,
        "mode": mode,
        "prompt": if prompt.is_empty() { None } else { Some(prompt) },
        "source_app": selection.app_name,
        "source_file": selection.file_path,
        "selection": {
            "text": selection.selected_text,
            "line_start": line_start,
            "line_end": line_end,
            "line_count": line_count,
        },
        "created_at": timestamp,
    });

    let content = serde_json::to_string_pretty(&job)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut file = fs::File::create(&job_file)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Render the job list panel
pub fn render_job_list(
    ui: &mut egui::Ui,
    cached_jobs: &[Job],
    selected_job_id: &mut Option<u64>,
) {
    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("JOBS").monospace().color(TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} total", cached_jobs.len()))
                        .small()
                        .color(TEXT_MUTED),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();

        // Job list
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Sort jobs: Running > Queued > Pending > Done/Failed/Merged
                let mut sorted_jobs = cached_jobs.to_vec();
                sorted_jobs.sort_by(|a, b| {
                    let priority = |s: JobStatus| match s {
                        JobStatus::Running => 0,
                        JobStatus::Queued => 1,
                        JobStatus::Pending => 2,
                        JobStatus::Done => 3,
                        JobStatus::Failed => 4,
                        JobStatus::Rejected => 5,
                        JobStatus::Merged => 6,
                    };
                    priority(a.status)
                        .cmp(&priority(b.status))
                        .then_with(|| b.created_at.cmp(&a.created_at))
                });

                for job in &sorted_jobs {
                    let is_selected = *selected_job_id == Some(job.id);
                    let bg = if is_selected {
                        BG_SELECTED
                    } else {
                        Color32::TRANSPARENT
                    };

                    let response = egui::Frame::none()
                        .fill(bg)
                        .inner_margin(egui::vec2(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Status indicator
                                let status_col = status_color(job.status);
                                ui.label(RichText::new("●").color(status_col));

                                // Job ID
                                ui.label(
                                    RichText::new(format!("#{}", job.id))
                                        .monospace()
                                        .color(TEXT_DIM),
                                );

                                // Mode
                                ui.label(RichText::new(&job.mode).monospace().color(TEXT_PRIMARY));

                                // Agent
                                ui.label(
                                    RichText::new(format!("[{}]", job.agent_id))
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            });

                            // Target (truncated)
                            let target = if job.target.len() > 40 {
                                format!("{}...", &job.target[..40])
                            } else {
                                job.target.clone()
                            };
                            ui.label(RichText::new(target).small().color(TEXT_DIM));
                        });

                    if response.response.interact(egui::Sense::click()).clicked() {
                        *selected_job_id = Some(job.id);
                    }
                }
            });
    });
}
