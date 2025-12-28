//! Status indicator animations

use eframe::egui::{Color32, Pos2, RichText, Sense, Ui, Vec2};

use super::PULSE_SPEED;

/// Pulsing indicator for running/active states
pub fn pulse_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let pulse = ((time * PULSE_SPEED).sin() * 0.5 + 0.5) as f32;

    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());

    let glow_color =
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (pulse * 80.0) as u8);
    ui.painter()
        .circle_filled(rect.center(), size * 0.8, glow_color);

    let inner_alpha = 150 + (pulse * 105.0) as u8;
    let inner_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), inner_alpha);
    ui.painter()
        .circle_filled(rect.center(), size * 0.4, inner_color);

    ui.ctx().request_repaint();
}

/// Spinner with custom color (animated)
pub fn colored_spinner(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());

    let center = rect.center();
    let radius = size * 0.4;

    let start_angle = (time * 4.0) as f32;
    let n_points = 20;

    for i in 0..n_points {
        let t = i as f32 / n_points as f32;
        let angle = start_angle + t * std::f32::consts::PI * 1.5;
        let alpha = (t * 255.0) as u8;
        let point_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

        let point = Pos2::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        );

        ui.painter().circle_filled(point, 2.0, point_color);
    }

    ui.ctx().request_repaint();
}

/// Animated lock icon for blocked status (pulsing lock brackets)
pub fn blocked_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let pulse = ((time * 1.5).sin() * 0.5 + 0.5) as f32;
    let alpha = 140 + (pulse * 115.0) as u8;
    let animated_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let text = RichText::new("[L]")
        .monospace()
        .color(animated_color)
        .size(size);
    ui.label(text);

    ui.ctx().request_repaint();
}

/// Animated dots for queued status (cycling through . .. ...)
pub fn queued_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let state = ((time * 2.5) as usize) % 4;
    let dots = match state {
        0 => "[   ]",
        1 => "[.  ]",
        2 => "[.. ]",
        _ => "[...]",
    };

    let text = RichText::new(dots).monospace().color(color).size(size);
    ui.label(text);

    ui.ctx().request_repaint();
}

/// Animated dot for pending status (gentle breathing pulse)
pub fn pending_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
    let alpha = 100 + (pulse * 155.0) as u8;
    let animated_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let text = RichText::new("[.]")
        .monospace()
        .color(animated_color)
        .size(size);
    ui.label(text);

    ui.ctx().request_repaint();
}
