//! Flow graph visualization for taint/data flow paths
//!
//! Displays a visual representation of the flow trace from source to sink.

use crate::bugbounty::{FlowKind, FlowTrace};
use egui::{Color32, RichText, Stroke, Ui};

/// Colors for different flow kinds
fn flow_kind_color(kind: FlowKind) -> Color32 {
    match kind {
        FlowKind::Taint => Color32::from_rgb(239, 68, 68),      // Red for taint
        FlowKind::Authz => Color32::from_rgb(249, 115, 22),     // Orange for authz
        FlowKind::Dataflow => Color32::from_rgb(59, 130, 246),  // Blue for dataflow
        FlowKind::Controlflow => Color32::from_rgb(34, 197, 94), // Green for control flow
    }
}

/// Render a flow trace as a vertical graph
pub fn render_flow_graph(ui: &mut Ui, trace: &FlowTrace) {
    if trace.edges.is_empty() {
        ui.label(RichText::new("No flow trace available").italics().color(Color32::GRAY));
        return;
    }

    ui.heading("Flow Trace");
    ui.add_space(8.0);

    // Legend
    ui.horizontal(|ui| {
        legend_item(ui, "Taint", FlowKind::Taint);
        legend_item(ui, "Auth", FlowKind::Authz);
        legend_item(ui, "Data", FlowKind::Dataflow);
        legend_item(ui, "Control", FlowKind::Controlflow);
    });

    ui.add_space(12.0);

    // Draw the flow nodes and edges
    let node_width = 200.0;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Collect unique nodes from edges
        let mut nodes = Vec::new();
        for (i, edge) in trace.edges.iter().enumerate() {
            if i == 0 {
                nodes.push(&edge.from);
            }
            nodes.push(&edge.to);
        }

        // Draw nodes and edges
        for (i, node) in nodes.iter().enumerate() {
            // Draw node
            let is_source = i == 0;
            let is_sink = i == nodes.len() - 1;
            let node_label = if is_source {
                "SOURCE"
            } else if is_sink {
                "SINK"
            } else {
                "intermediate"
            };

            let border_color = if is_source {
                Color32::from_rgb(239, 68, 68) // Red for source
            } else if is_sink {
                Color32::from_rgb(220, 38, 38) // Darker red for sink
            } else {
                Color32::from_rgb(100, 100, 120)
            };

            let frame = egui::Frame::new()
                .fill(Color32::from_rgb(40, 40, 55))
                .stroke(Stroke::new(2.0, border_color))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(8));

            frame.show(ui, |ui| {
                ui.set_min_width(node_width);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(node_label)
                            .small()
                            .color(border_color)
                            .strong(),
                    );
                });

                // File path
                ui.label(
                    RichText::new(&node.file)
                        .small()
                        .color(Color32::from_rgb(150, 180, 220)),
                );

                // Line number
                if let Some(line) = node.line {
                    ui.label(
                        RichText::new(format!("Line {}", line))
                            .small()
                            .color(Color32::GRAY),
                    );
                }

                // Symbol/snippet
                if let Some(ref symbol) = node.symbol {
                    ui.label(
                        RichText::new(symbol)
                            .small()
                            .monospace()
                            .color(Color32::from_rgb(180, 180, 200)),
                    );
                }

                if let Some(ref snippet) = node.snippet {
                    let display_snippet = if snippet.len() > 50 {
                        format!("{}...", &snippet[..47])
                    } else {
                        snippet.clone()
                    };
                    ui.label(
                        RichText::new(display_snippet)
                            .small()
                            .monospace()
                            .color(Color32::from_rgb(160, 160, 180)),
                    );
                }
            });

            // Draw edge arrow to next node (if not last)
            if i < nodes.len() - 1 {
                let edge = &trace.edges[i];
                let color = flow_kind_color(edge.kind);

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.add_space(node_width / 2.0 - 20.0);

                    // Arrow line
                    ui.vertical(|ui| {
                        ui.label(RichText::new("│").color(color));
                        ui.label(RichText::new("│").color(color));
                        ui.label(RichText::new("▼").color(color));
                    });

                    // Edge label
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(edge.kind.as_str())
                            .small()
                            .color(color),
                    );

                    if let Some(ref notes) = edge.notes {
                        ui.label(
                            RichText::new(format!("({})", notes))
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
                });

                ui.add_space(4.0);
            }
        }
    });
}

fn legend_item(ui: &mut Ui, label: &str, kind: FlowKind) {
    let color = flow_kind_color(kind);
    ui.horizontal(|ui| {
        ui.colored_label(color, "●");
        ui.label(RichText::new(label).small().color(Color32::GRAY));
    });
    ui.add_space(8.0);
}

/// Render a compact inline flow summary (for finding cards)
pub fn render_flow_summary(ui: &mut Ui, trace: &FlowTrace) {
    if trace.edges.is_empty() {
        return;
    }

    let summary = trace.summary();
    let color = if trace.edges.iter().any(|e| e.kind == FlowKind::Taint) {
        Color32::from_rgb(239, 68, 68)
    } else {
        Color32::from_rgb(59, 130, 246)
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new("⤷").color(color));
        ui.label(
            RichText::new(&summary)
                .small()
                .color(Color32::GRAY),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bugbounty::CodeLocation;

    #[test]
    fn test_flow_kind_color() {
        let taint_color = flow_kind_color(FlowKind::Taint);
        assert_eq!(taint_color.r(), 239); // Red channel
    }
}
