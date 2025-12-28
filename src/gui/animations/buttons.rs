//! Button animation components

use eframe::egui::{self, Color32, Id, Response, RichText, Ui};

use super::lerp_color;

/// Animated button with hover glow effect
pub fn animated_button(
    ui: &mut Ui,
    text: impl Into<RichText>,
    base_color: Color32,
    id_salt: impl std::hash::Hash,
) -> Response {
    let text = text.into();
    let id = Id::new(id_salt);

    let was_hovered = ui
        .ctx()
        .memory(|mem| mem.data.get_temp::<bool>(id).unwrap_or(false));

    let hover_anim = ui
        .ctx()
        .animate_bool_with_time(id.with("anim"), was_hovered, 0.15);

    let glow_alpha = (hover_anim * 40.0) as u8;
    let fill_color =
        Color32::from_rgba_unmultiplied(base_color.r(), base_color.g(), base_color.b(), glow_alpha);
    let stroke_alpha = (hover_anim * 0.6 * 255.0) as u8;
    let stroke_color = Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(),
        base_color.b(),
        stroke_alpha,
    );

    let button = egui::Button::new(text.color(base_color))
        .fill(fill_color)
        .stroke(egui::Stroke::new(1.0, stroke_color));

    let response = ui.add(button);

    let is_hovered = response.hovered();
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    if is_hovered != was_hovered || hover_anim > 0.01 && hover_anim < 0.99 {
        ui.ctx().request_repaint();
    }

    response
}

/// Animated icon button (smaller, icon-only)
pub fn animated_icon_button(
    ui: &mut Ui,
    icon: &str,
    base_color: Color32,
    hover_color: Color32,
    id_salt: impl std::hash::Hash,
) -> Response {
    let id = Id::new(id_salt);

    let was_hovered = ui
        .ctx()
        .memory(|mem| mem.data.get_temp::<bool>(id).unwrap_or(false));

    let hover_progress = ui
        .ctx()
        .animate_bool_with_time(id.with("anim"), was_hovered, 0.12);
    let current_color = lerp_color(base_color, hover_color, hover_progress);

    let response = ui.add(
        egui::Button::new(RichText::new(icon).color(current_color))
            .fill(Color32::TRANSPARENT)
            .frame(false),
    );

    let is_hovered = response.hovered();
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    if is_hovered != was_hovered || hover_progress > 0.01 && hover_progress < 0.99 {
        ui.ctx().request_repaint();
    }

    response
}
