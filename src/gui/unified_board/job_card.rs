//! Job card rendering for the Unified Board

use super::UnifiedBoardState;
use crate::{Job, JobStatus};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};

/// Render a job card in the unified board
pub fn render_job_card(ui: &mut Ui, state: &mut UnifiedBoardState, job: &Job) {
    let is_selected = state.selected_job == Some(job.id);
    let is_dragging = state.dragged_job_id == Some(job.id);

    // Card frame with status-based coloring
    let status_color = job_status_color(job.status);
    let frame = Frame::new()
        .fill(if is_selected {
            Color32::from_rgb(45, 55, 72)
        } else if is_dragging {
            Color32::from_rgb(35, 45, 62)
        } else {
            Color32::from_rgb(30, 35, 45)
        })
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(
            if is_selected { 2.0 } else { 1.0 },
            if is_selected {
                status_color
            } else {
                Color32::from_rgb(55, 65, 75)
            },
        ))
        .inner_margin(Margin::same(8));

    let response = frame
        .show(ui, |ui| {
            ui.set_min_width(160.0);

            // Header: Job ID + Status indicator
            ui.horizontal(|ui| {
                // Status indicator (circle or spinner)
                let indicator = match job.status {
                    JobStatus::Running => "\u{25B6}", // Play symbol
                    JobStatus::Queued => "\u{23F3}",  // Hourglass
                    JobStatus::Pending => "\u{25CB}", // Circle
                    JobStatus::Done => "\u{2713}",    // Check
                    JobStatus::Failed => "\u{2717}",  // X
                    JobStatus::Blocked => "\u{23F8}", // Pause
                    JobStatus::Rejected => "\u{2212}", // Minus
                    JobStatus::Merged => "\u{2714}",  // Heavy check
                };
                ui.label(
                    RichText::new(indicator)
                        .color(status_color)
                        .size(12.0),
                );

                ui.label(
                    RichText::new(format!("#{}", job.id))
                        .strong()
                        .size(12.0)
                        .color(Color32::WHITE),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Agent badge
                    ui.label(
                        RichText::new(&job.agent_id)
                            .small()
                            .color(agent_color(&job.agent_id)),
                    );
                });
            });

            // Skill/Mode
            ui.add_space(2.0);
            ui.label(
                RichText::new(&job.skill)
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 200)),
            );

            // Target (truncated)
            if !job.target.is_empty() {
                ui.label(
                    RichText::new(truncate(&job.target, 30))
                        .small()
                        .color(Color32::GRAY),
                );
            }

            // Metadata row
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Chain badge
                if job.skill.contains("chain") || !job.chain_step_history.is_empty() {
                    ui.label(
                        RichText::new("CHAIN")
                            .small()
                            .color(Color32::from_rgb(168, 85, 247))
                            .background_color(Color32::from_rgb(168, 85, 247).linear_multiply(0.2)),
                    );
                }

                // Duration
                if let Some(stats) = &job.stats {
                    if let Some(duration) = stats.duration {
                        ui.label(
                            RichText::new(format_duration(duration))
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
                }

                // Linked findings count
                let linked_count = job.bugbounty_finding_ids.len();
                if linked_count > 0 {
                    ui.label(
                        RichText::new(format!("{} findings", linked_count))
                            .small()
                            .color(Color32::from_rgb(52, 211, 153)),
                    );
                }
            });

            // Result state (if available)
            if let Some(ref result) = job.result {
                if let Some(ref state_str) = result.state {
                    ui.label(
                        RichText::new(state_str)
                            .small()
                            .color(result_state_color(state_str)),
                    );
                }
            }
        })
        .response;

    // Enable drag sensing on the response
    let response = response.interact(egui::Sense::click_and_drag());

    // Handle interactions
    if response.clicked() {
        state.selected_job = Some(job.id);
        state.selected_finding = None;
    }

    // Drag start
    if response.drag_started() {
        state.dragged_job_id = Some(job.id);
    }

    // Show drag cursor when dragging
    if is_dragging {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    // Context menu
    response.context_menu(|ui| {
        if ui.button("View Details").clicked() {
            state.selected_job = Some(job.id);
            ui.close();
        }

        ui.separator();

        match job.status {
            JobStatus::Pending => {
                if ui.button("Queue").clicked() {
                    // Will be handled by parent
                    ui.close();
                }
            }
            JobStatus::Queued | JobStatus::Running => {
                if ui.button("Kill").clicked() {
                    // Will be handled by parent
                    ui.close();
                }
            }
            JobStatus::Done | JobStatus::Failed | JobStatus::Rejected => {
                if ui.button("Restart").clicked() {
                    // Will be handled by parent
                    ui.close();
                }
            }
            _ => {}
        }
    });
}

/// Get color for job status
pub fn job_status_color(status: JobStatus) -> Color32 {
    match status {
        JobStatus::Running => Color32::from_rgb(59, 130, 246),   // Blue
        JobStatus::Queued => Color32::from_rgb(168, 85, 247),    // Purple
        JobStatus::Pending => Color32::from_rgb(156, 163, 175),  // Gray
        JobStatus::Done => Color32::from_rgb(34, 197, 94),       // Green
        JobStatus::Failed => Color32::from_rgb(239, 68, 68),     // Red
        JobStatus::Blocked => Color32::from_rgb(251, 191, 36),   // Yellow
        JobStatus::Rejected => Color32::from_rgb(107, 114, 128), // Dark gray
        JobStatus::Merged => Color32::from_rgb(16, 185, 129),    // Teal
    }
}

/// Get color for agent (hash-based)
fn agent_color(agent_id: &str) -> Color32 {
    let hash: u32 = agent_id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32).wrapping_mul(31));
    let hue = (hash % 360) as f32;
    let (r, g, b) = hsl_to_rgb(hue, 0.6, 0.65);
    Color32::from_rgb(r, g, b)
}

/// Get color for result state
fn result_state_color(state: &str) -> Color32 {
    let state_lower = state.to_lowercase();
    if state_lower.contains("success") || state_lower.contains("done") || state_lower.contains("pass") {
        Color32::from_rgb(34, 197, 94)
    } else if state_lower.contains("fail") || state_lower.contains("error") {
        Color32::from_rgb(239, 68, 68)
    } else if state_lower.contains("skip") || state_lower.contains("partial") {
        Color32::from_rgb(251, 191, 36)
    } else {
        Color32::GRAY
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
