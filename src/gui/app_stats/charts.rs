//! Native egui chart rendering
//!
//! Ring/donut charts for the Dashboard V2.

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use std::f32::consts::PI;

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, TEXT_DIM, TEXT_PRIMARY};
use crate::stats::{AgentStats, TokenBreakdown};

// ============================================================================
// Dashboard V2 Charts - Ring/Donut Charts
// ============================================================================

/// Ring chart segment data
struct RingSegment {
    label: String,
    value: f64,
    color: Color32,
    detail: String, // e.g., "8 jobs" or "$0.10"
}

/// Draw a ring/donut chart with legend
fn ring_chart_internal(ui: &mut egui::Ui, segments: &[RingSegment], size: f32) {
    if segments.is_empty() {
        ui.label(RichText::new("No data").color(TEXT_DIM));
        return;
    }

    let total: f64 = segments.iter().map(|s| s.value).sum();
    if total <= 0.0 {
        ui.label(RichText::new("No data").color(TEXT_DIM));
        return;
    }

    ui.horizontal(|ui| {
        // Ring chart on the left
        let ring_size = size.min(120.0);
        let (response, painter) = ui.allocate_painter(Vec2::splat(ring_size), egui::Sense::hover());
        let rect = response.rect;
        let center = rect.center();
        let outer_radius = ring_size / 2.0 - 4.0;
        let inner_radius = outer_radius * 0.6;

        let mut start_angle = -PI / 2.0; // Start at top
        for segment in segments {
            let sweep = (segment.value / total) as f32 * 2.0 * PI;
            if sweep > 0.01 {
                draw_arc(&painter, center, inner_radius, outer_radius, start_angle, sweep, segment.color);
            }
            start_angle += sweep;
        }

        // Legend on the right
        ui.vertical(|ui| {
            for segment in segments {
                let pct = (segment.value / total * 100.0) as u32;
                ui.horizontal(|ui| {
                    // Color box
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, segment.color);
                    // Label + percentage
                    ui.label(RichText::new(format!("{} {}%", segment.label, pct)).small().color(TEXT_PRIMARY));
                });
                // Detail line
                ui.label(RichText::new(&segment.detail).small().color(TEXT_DIM));
            }
        });
    });
}

/// Draw an arc segment (for ring chart)
fn draw_arc(painter: &egui::Painter, center: Pos2, inner_r: f32, outer_r: f32, start: f32, sweep: f32, color: Color32) {
    // Approximate arc with triangles
    let steps = ((sweep.abs() / (PI / 18.0)).ceil() as usize).max(3);
    let step_angle = sweep / steps as f32;

    for i in 0..steps {
        let a1 = start + i as f32 * step_angle;
        let a2 = start + (i + 1) as f32 * step_angle;

        let outer1 = Pos2::new(center.x + outer_r * a1.cos(), center.y + outer_r * a1.sin());
        let outer2 = Pos2::new(center.x + outer_r * a2.cos(), center.y + outer_r * a2.sin());
        let inner1 = Pos2::new(center.x + inner_r * a1.cos(), center.y + inner_r * a1.sin());
        let inner2 = Pos2::new(center.x + inner_r * a2.cos(), center.y + inner_r * a2.sin());

        // Two triangles to form the arc segment
        painter.add(egui::Shape::convex_polygon(vec![outer1, outer2, inner2, inner1], color, Stroke::NONE));
    }
}

/// Agent ring chart showing Claude vs Codex distribution
pub fn agent_ring_chart(ui: &mut egui::Ui, agents: &[AgentStats], size: f32) {
    let colors = [ACCENT_CYAN, ACCENT_PURPLE, ACCENT_GREEN, ACCENT_YELLOW];
    let segments: Vec<RingSegment> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| RingSegment {
            label: a.name.clone(),
            value: a.jobs as f64,
            color: colors[i % colors.len()],
            detail: format!("{} jobs, ${:.2}", a.succeeded_jobs, a.cost_usd),
        })
        .collect();
    ring_chart_internal(ui, &segments, size);
}

/// Token type ring chart showing Input/Output/Cache distribution
pub fn token_ring_chart(ui: &mut egui::Ui, tokens: &TokenBreakdown, size: f32) {
    let segments = vec![
        RingSegment {
            label: "Input".to_string(),
            value: tokens.input as f64,
            color: ACCENT_CYAN,
            detail: format_tokens(tokens.input),
        },
        RingSegment {
            label: "Output".to_string(),
            value: tokens.output as f64,
            color: ACCENT_PURPLE,
            detail: format_tokens(tokens.output),
        },
        RingSegment {
            label: "Cache".to_string(),
            value: tokens.total_cache() as f64,
            color: ACCENT_GREEN,
            detail: format!("{} ({}% hit)", format_tokens(tokens.total_cache()), tokens.cache_hit_rate() as u32),
        },
    ];
    ring_chart_internal(ui, &segments, size);
}

/// Format token count for display
pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Format bytes for display
pub fn format_bytes(bytes: f64) -> String {
    if bytes >= 1_000_000.0 {
        format!("{:.1} MB", bytes / 1_000_000.0)
    } else if bytes >= 1_000.0 {
        format!("{:.1} KB", bytes / 1_000.0)
    } else {
        format!("{:.0} B", bytes)
    }
}

/// Format duration for display
pub fn format_duration(ms: f64) -> String {
    if ms >= 60_000.0 {
        format!("{:.1}m", ms / 60_000.0)
    } else if ms >= 1_000.0 {
        format!("{:.1}s", ms / 1_000.0)
    } else {
        format!("{:.0}ms", ms)
    }
}
