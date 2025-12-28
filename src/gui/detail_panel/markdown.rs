//! Markdown rendering helpers for the detail panel

use eframe::egui;

use crate::gui::theme::{ACCENT_CYAN, BG_HIGHLIGHT, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Apply CRT theme visuals for markdown rendering
#[inline]
pub(super) fn apply_markdown_theme(ui: &mut egui::Ui) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.override_text_color = Some(TEXT_DIM);
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.hyperlink_color = ACCENT_CYAN;
    visuals.code_bg_color = BG_HIGHLIGHT;
    visuals.extreme_bg_color = BG_HIGHLIGHT;
    visuals.widgets.active.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;
}

#[inline]
fn apply_markdown_theme_with_text_color(ui: &mut egui::Ui, text_color: egui::Color32) {
    apply_markdown_theme(ui);
    ui.style_mut().visuals.override_text_color = Some(text_color);
}

pub(super) fn render_markdown_inline_colored(
    ui: &mut egui::Ui,
    text: &str,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    text_color: egui::Color32,
) {
    ui.scope(|ui| {
        apply_markdown_theme_with_text_color(ui, text_color);
        egui_commonmark::CommonMarkViewer::new().show(ui, commonmark_cache, text);
    });
}

/// Render markdown content with themed scroll area
pub(super) fn render_markdown_scroll(
    ui: &mut egui::Ui,
    text: &str,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
) {
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.scope(|ui| {
                apply_markdown_theme(ui);
                egui_commonmark::CommonMarkViewer::new().show(ui, commonmark_cache, text);
            });
        });
}
