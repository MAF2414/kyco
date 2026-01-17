//! Finding card rendering for the Unified Board

use super::UnifiedBoardState;
use crate::bugbounty::{Finding, FindingStatus, Severity};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};

/// Render a finding card in the unified board
pub fn render_finding_card(ui: &mut Ui, state: &mut UnifiedBoardState, finding: &Finding) {
    let is_selected = state.selected_finding.as_ref() == Some(&finding.id);
    let is_dragging = state.dragged_finding_id.as_ref() == Some(&finding.id);

    // Card frame
    let frame = Frame::new()
        .fill(if is_selected {
            Color32::from_rgb(50, 50, 70)
        } else if is_dragging {
            Color32::from_rgb(40, 40, 60)
        } else {
            Color32::from_rgb(35, 35, 45)
        })
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(
            if is_selected { 2.0 } else { 1.0 },
            if is_selected {
                severity_color(finding.severity)
            } else {
                Color32::from_rgb(60, 60, 70)
            },
        ))
        .inner_margin(Margin::same(8));

    let response = frame
        .show(ui, |ui| {
            ui.set_min_width(160.0);

            // Header: ID + Severity badge
            ui.horizontal(|ui| {
                ui.label(RichText::new(&finding.id).strong().size(12.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(severity) = finding.severity {
                        let (text, color) = severity_badge(severity);
                        ui.label(
                            RichText::new(text)
                                .color(color)
                                .background_color(color.linear_multiply(0.2))
                                .size(10.0),
                        );
                    }
                });
            });

            // Title
            ui.add_space(4.0);
            ui.label(RichText::new(truncate(&finding.title, 35)).size(12.0));

            // Metadata row
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Confidence indicator
                if let Some(confidence) = finding.confidence {
                    ui.label(
                        RichText::new(format!(
                            "{}",
                            confidence.as_str().chars().next().unwrap_or('?').to_uppercase()
                        ))
                        .small()
                        .color(Color32::GRAY),
                    );
                }

                // Asset count
                if !finding.affected_assets.is_empty() {
                    ui.label(
                        RichText::new(format!("{} assets", finding.affected_assets.len()))
                            .small()
                            .color(Color32::GRAY),
                    );
                }

                // Linked jobs count
                let job_count = state.get_linked_job_count(&finding.id);
                if job_count > 0 {
                    ui.label(
                        RichText::new(format!("{} jobs", job_count))
                            .small()
                            .color(Color32::from_rgb(59, 130, 246)),
                    );
                }

                // CWE
                if let Some(ref cwe) = finding.cwe_id {
                    ui.label(
                        RichText::new(cwe)
                            .small()
                            .color(Color32::from_rgb(100, 150, 200)),
                    );
                }
            });
        })
        .response;

    // Enable drag sensing on the response
    let response = response.interact(egui::Sense::click_and_drag());

    // Handle interactions
    if response.clicked() {
        state.selected_finding = Some(finding.id.clone());
        state.selected_job = None;
    }

    // Drag start
    if response.drag_started() {
        state.dragged_finding_id = Some(finding.id.clone());
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
            state.selected_finding = Some(finding.id.clone());
            ui.close();
        }

        ui.separator();

        if ui.button("Mark Verified").clicked() {
            state.move_finding(&finding.id, FindingStatus::Verified);
            ui.close();
        }

        if ui.button("Mark FP").clicked() {
            state.show_fp_dialog = true;
            state.fp_target_finding_id = Some(finding.id.clone());
            state.fp_reason_input = finding
                .fp_reason
                .clone()
                .unwrap_or_else(|| "false positive".to_string());
            ui.close();
        }

        ui.separator();

        if ui.button("Export Markdown").clicked() {
            // Will be handled by parent
            ui.close();
        }
    });
}

/// Get severity badge text and color
fn severity_badge(severity: Severity) -> (&'static str, Color32) {
    match severity {
        Severity::Critical => ("CRIT", Color32::from_rgb(220, 38, 38)),
        Severity::High => ("HIGH", Color32::from_rgb(249, 115, 22)),
        Severity::Medium => ("MED", Color32::from_rgb(234, 179, 8)),
        Severity::Low => ("LOW", Color32::from_rgb(34, 197, 94)),
        Severity::Info => ("INFO", Color32::from_rgb(59, 130, 246)),
    }
}

/// Get color for severity level
pub fn severity_color(severity: Option<Severity>) -> Color32 {
    match severity {
        Some(Severity::Critical) => Color32::from_rgb(220, 38, 38),
        Some(Severity::High) => Color32::from_rgb(249, 115, 22),
        Some(Severity::Medium) => Color32::from_rgb(234, 179, 8),
        Some(Severity::Low) => Color32::from_rgb(34, 197, 94),
        Some(Severity::Info) => Color32::from_rgb(59, 130, 246),
        None => Color32::GRAY,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
