//! Native egui chart rendering
//!
//! Simple bar, line, and ring charts using egui's Painter API.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use std::f32::consts::PI;

use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_PRIMARY,
};
use crate::stats::{AgentStats, DailyStatsView, TokenBreakdown};

const PADDING: f32 = 8.0;
const BAR_GAP: f32 = 4.0;
const LABEL_HEIGHT: f32 = 16.0;

/// Vertical bar chart for time series data
pub fn bar_chart<F>(ui: &mut egui::Ui, data: &[DailyStatsView], height: f32, get_value: F, color: Color32)
where
    F: Fn(&DailyStatsView) -> f64,
{
    if data.is_empty() {
        ui.label(RichText::new("No data available").color(TEXT_DIM));
        return;
    }
    let values: Vec<f64> = data.iter().map(&get_value).collect();
    let max_val = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);

    let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, BG_SECONDARY);

    let chart_rect = rect.shrink(PADDING);
    let bar_area_height = chart_rect.height() - LABEL_HEIGHT;
    let n = values.len() as f32;
    let bar_width = ((chart_rect.width() - BAR_GAP * (n - 1.0)) / n).max(2.0);

    for (i, (val, day)) in values.iter().zip(data.iter()).enumerate() {
        let x = chart_rect.left() + i as f32 * (bar_width + BAR_GAP);
        let bar_height = ((*val / max_val) as f32 * bar_area_height).max(1.0);
        let bar_rect = Rect::from_min_size(
            Pos2::new(x, chart_rect.top() + bar_area_height - bar_height),
            Vec2::new(bar_width, bar_height),
        );
        painter.rect_filled(bar_rect, 2.0, color);

        // X-axis label (every few bars to avoid crowding)
        if i % ((n as usize / 7).max(1)) == 0 {
            let short_day = if day.day.len() > 5 { &day.day[5..] } else { &day.day };
            painter.text(
                Pos2::new(x + bar_width / 2.0, chart_rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                short_day,
                egui::FontId::proportional(9.0),
                TEXT_DIM,
            );
        }
    }
}

/// Line chart for time series data
pub fn line_chart<F>(ui: &mut egui::Ui, data: &[DailyStatsView], height: f32, get_value: F, color: Color32)
where
    F: Fn(&DailyStatsView) -> f64,
{
    if data.is_empty() {
        ui.label(RichText::new("No data available").color(TEXT_DIM));
        return;
    }
    let values: Vec<f64> = data.iter().map(&get_value).collect();
    let max_val = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);

    let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, BG_SECONDARY);

    let chart_rect = rect.shrink(PADDING);
    let chart_height = chart_rect.height() - LABEL_HEIGHT;
    let n = values.len();
    if n < 2 {
        return;
    }
    let step_x = chart_rect.width() / (n - 1) as f32;

    // Draw line segments
    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let x = chart_rect.left() + i as f32 * step_x;
            let y = chart_rect.top() + chart_height - ((*val / max_val) as f32 * chart_height);
            Pos2::new(x, y)
        })
        .collect();

    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(2.0, color));
    }

    // Draw points
    for point in &points {
        painter.circle_filled(*point, 3.0, color);
    }

    // X-axis labels
    for (i, day) in data.iter().enumerate() {
        if i % ((n / 7).max(1)) == 0 {
            let x = chart_rect.left() + i as f32 * step_x;
            let short_day = if day.day.len() > 5 { &day.day[5..] } else { &day.day };
            painter.text(
                Pos2::new(x, chart_rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                short_day,
                egui::FontId::proportional(9.0),
                TEXT_DIM,
            );
        }
    }
}

/// Token usage chart with separate lines for input/output
pub fn token_chart(ui: &mut egui::Ui, data: &[DailyStatsView], height: f32) {
    if data.is_empty() {
        ui.label(RichText::new("No data available").color(TEXT_DIM));
        return;
    }
    let input: Vec<f64> = data.iter().map(|d| d.total_input_tokens as f64).collect();
    let output: Vec<f64> = data.iter().map(|d| d.total_output_tokens as f64).collect();
    let max_val = input.iter().chain(output.iter()).cloned().fold(0.0f64, f64::max).max(1.0);

    let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, BG_SECONDARY);

    let chart_rect = rect.shrink(PADDING);
    let chart_height = chart_rect.height() - LABEL_HEIGHT - 14.0; // Extra space for legend
    let n = data.len();
    if n < 2 { return; }
    let step_x = chart_rect.width() / (n - 1) as f32;

    // Draw both lines
    for (values, color, label) in [(&input, ACCENT_CYAN, "In"), (&output, ACCENT_PURPLE, "Out")] {
        let points: Vec<Pos2> = values.iter().enumerate().map(|(i, val)| {
            let x = chart_rect.left() + i as f32 * step_x;
            let y = chart_rect.top() + chart_height - ((*val / max_val) as f32 * chart_height);
            Pos2::new(x, y)
        }).collect();
        for window in points.windows(2) {
            painter.line_segment([window[0], window[1]], Stroke::new(2.0, color));
        }
        for point in &points {
            painter.circle_filled(*point, 2.5, color);
        }
        // Legend entry
        let legend_x = if label == "In" { chart_rect.left() + 5.0 } else { chart_rect.left() + 55.0 };
        painter.rect_filled(Rect::from_min_size(Pos2::new(legend_x, chart_rect.top() + 2.0), Vec2::new(10.0, 10.0)), 1.0, color);
        painter.text(Pos2::new(legend_x + 14.0, chart_rect.top() + 7.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(9.0), TEXT_PRIMARY);
    }

    // X-axis labels
    for (i, day) in data.iter().enumerate() {
        if i % ((n / 7).max(1)) == 0 {
            let x = chart_rect.left() + i as f32 * step_x;
            let short_day = if day.day.len() > 5 { &day.day[5..] } else { &day.day };
            painter.text(Pos2::new(x, chart_rect.bottom() - 2.0), egui::Align2::CENTER_BOTTOM, short_day, egui::FontId::proportional(9.0), TEXT_DIM);
        }
    }
}

/// Horizontal bar chart for categorical data (modes, tools)
pub fn horizontal_bars(ui: &mut egui::Ui, data: &[(String, u64)], height: f32, color: Color32) {
    if data.is_empty() {
        ui.label(RichText::new("No data available").color(TEXT_DIM));
        return;
    }
    let max_val = data.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;

    let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, BG_SECONDARY);

    let chart_rect = rect.shrink(PADDING);
    let label_width = 80.0;
    let bar_area_width = chart_rect.width() - label_width - 50.0;
    let n = data.len().min(8);
    let bar_height = ((chart_rect.height() - BAR_GAP * (n as f32 - 1.0)) / n as f32).max(12.0);

    for (i, (name, count)) in data.iter().take(n).enumerate() {
        let y = chart_rect.top() + i as f32 * (bar_height + BAR_GAP);
        // Label
        painter.text(Pos2::new(chart_rect.left(), y + bar_height / 2.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(11.0), TEXT_PRIMARY);
        // Bar
        let bar_width = ((*count as f64 / max_val) as f32 * bar_area_width).max(2.0);
        let bar_rect = Rect::from_min_size(Pos2::new(chart_rect.left() + label_width, y), Vec2::new(bar_width, bar_height));
        painter.rect_filled(bar_rect, 2.0, color);
        // Count
        painter.text(Pos2::new(chart_rect.right() - 5.0, y + bar_height / 2.0), egui::Align2::RIGHT_CENTER, count.to_string(), egui::FontId::proportional(11.0), color);
    }
}

/// Agent comparison bars (claude vs codex) - legacy
pub fn agent_bars(ui: &mut egui::Ui, data: &[(String, u64, u64)], height: f32) {
    if data.is_empty() {
        ui.label(RichText::new("No agent data").color(TEXT_DIM));
        return;
    }
    let bars: Vec<(String, u64)> = data.iter().map(|(name, count, _)| (name.clone(), *count)).collect();
    let colors = [ACCENT_CYAN, ACCENT_PURPLE];
    let max_val = bars.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;

    let (response, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, BG_SECONDARY);

    let chart_rect = rect.shrink(PADDING);
    let label_width = 80.0;
    let bar_area_width = chart_rect.width() - label_width - 50.0;
    let n = bars.len().min(4);
    let bar_height = ((chart_rect.height() - BAR_GAP * (n as f32 - 1.0)) / n as f32).max(20.0);

    for (i, (name, count)) in bars.iter().take(n).enumerate() {
        let y = chart_rect.top() + i as f32 * (bar_height + BAR_GAP);
        let color = colors[i % colors.len()];
        painter.text(Pos2::new(chart_rect.left(), y + bar_height / 2.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), TEXT_PRIMARY);
        let bar_width = ((*count as f64 / max_val) as f32 * bar_area_width).max(2.0);
        let bar_rect = Rect::from_min_size(Pos2::new(chart_rect.left() + label_width, y), Vec2::new(bar_width, bar_height));
        painter.rect_filled(bar_rect, 2.0, color);
        painter.text(Pos2::new(chart_rect.right() - 5.0, y + bar_height / 2.0), egui::Align2::RIGHT_CENTER, count.to_string(), egui::FontId::proportional(12.0), color);
    }
}

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
