//! Finding card rendering for Kanban board

use super::{severity_color, KanbanState};
use crate::bugbounty::{Finding, Severity};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};

/// Render a finding card in the Kanban column
pub fn render_finding_card(ui: &mut Ui, state: &mut KanbanState, finding: &Finding) {
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
            1.0,
            if is_selected {
                severity_color(finding.severity)
            } else {
                Color32::from_rgb(60, 60, 70)
            },
        ))
        .inner_margin(Margin::same(8));

    let response = frame
        .show(ui, |ui| {
            ui.set_min_width(180.0);

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
            ui.label(
                RichText::new(truncate(&finding.title, 40))
                    .size(13.0),
            );

            // Metadata row
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Confidence indicator
                if let Some(confidence) = finding.confidence {
                    ui.label(
                        RichText::new(format!("{}", confidence.as_str().chars().next().unwrap_or('?').to_uppercase()))
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
                if let Some(count) = state.job_count_by_finding.get(&finding.id).copied() {
                    if count > 0 {
                        ui.label(
                            RichText::new(format!("{} jobs", count))
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
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

    // Handle interactions
    if response.clicked() {
        state.selected_finding = Some(finding.id.clone());
    }

    // Drag start
    if response.drag_started() {
        state.dragged_finding_id = Some(finding.id.clone());
    }

    // Drag end (handled in column)
    if response.drag_stopped() && state.dragged_finding_id.as_ref() == Some(&finding.id) {
        // Will be handled by drop zone
    }

    // Context menu
    response.context_menu(|ui| {
        if ui.button("View Details").clicked() {
            state.selected_finding = Some(finding.id.clone());
            ui.close();
        }

        ui.separator();

        if ui.button("Mark Verified").clicked() {
            state.move_finding(&finding.id, crate::bugbounty::FindingStatus::Verified);
            ui.close();
        }

        if ui.button("Mark FP").clicked() {
            state.move_finding(&finding.id, crate::bugbounty::FindingStatus::FalsePositive);
            ui.close();
        }

        ui.separator();

        if ui.button("Export Markdown").clicked() {
            state.export_finding(&finding.id, "markdown");
            ui.close();
        }

        if ui.button("Export HackerOne").clicked() {
            state.export_finding(&finding.id, "hackerone");
            ui.close();
        }
    });
}

fn severity_badge(severity: Severity) -> (&'static str, Color32) {
    match severity {
        Severity::Critical => ("CRIT", Color32::from_rgb(220, 38, 38)),
        Severity::High => ("HIGH", Color32::from_rgb(249, 115, 22)),
        Severity::Medium => ("MED", Color32::from_rgb(234, 179, 8)),
        Severity::Low => ("LOW", Color32::from_rgb(34, 197, 94)),
        Severity::Info => ("INFO", Color32::from_rgb(59, 130, 246)),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
