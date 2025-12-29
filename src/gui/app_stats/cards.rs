//! Summary card widgets for the dashboard

use eframe::egui::{self, RichText};

use crate::gui::theme::{ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, TEXT_DIM};

/// Render a summary card with u64 value and trend indicator
pub fn summary_card_full<F>(
    ui: &mut egui::Ui,
    label: &str,
    value: u64,
    trend: &crate::stats::TrendValue,
    format: F,
    value_color: egui::Color32,
    width: f32,
    invert_trend: bool,
) where
    F: Fn(u64) -> String,
{
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(width - 24.0); // Account for inner margin
            ui.vertical(|ui| {
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                ui.label(RichText::new(format(value)).size(18.0).color(value_color));
                if invert_trend {
                    render_trend_inverted(ui, trend);
                } else {
                    render_trend(ui, trend);
                }
            });
        });
}

/// Render a summary card with f64 value and trend indicator
pub fn summary_card_full_f64<F>(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    trend: &crate::stats::TrendValue,
    format: F,
    value_color: egui::Color32,
    width: f32,
    invert_trend: bool,
) where
    F: Fn(f64) -> String,
{
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(width - 24.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(label).small().color(TEXT_DIM));
                ui.label(RichText::new(format(value)).size(18.0).color(value_color));
                if invert_trend {
                    render_trend_inverted(ui, trend);
                } else {
                    render_trend(ui, trend);
                }
            });
        });
}

/// Render trend indicator (up = green, down = red)
fn render_trend(ui: &mut egui::Ui, trend: &crate::stats::TrendValue) {
    let pct = trend.percent_change();
    if pct.abs() < 0.1 {
        ui.label(RichText::new("—").small().color(TEXT_DIM));
    } else {
        let (prefix, color) = if pct > 0.0 {
            ("▲", ACCENT_GREEN)
        } else {
            ("▼", ACCENT_RED)
        };
        ui.label(RichText::new(format!("{}{:.0}%", prefix, pct.abs())).small().color(color));
    }
}

/// Render inverted trend indicator (up = red, down = green)
fn render_trend_inverted(ui: &mut egui::Ui, trend: &crate::stats::TrendValue) {
    let pct = trend.percent_change();
    if pct.abs() < 0.1 {
        ui.label(RichText::new("—").small().color(TEXT_DIM));
    } else {
        // Inverted: up is bad (red), down is good (green)
        let (prefix, color) = if pct > 0.0 {
            ("▲", ACCENT_RED)
        } else {
            ("▼", ACCENT_GREEN)
        };
        ui.label(RichText::new(format!("{}{:.0}%", prefix, pct.abs())).small().color(color));
    }
}
