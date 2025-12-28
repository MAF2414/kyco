//! Animation utilities for the GUI
//!
//! Provides smooth transitions and visual feedback through:
//! - Hover effects for interactive elements
//! - View transition animations
//! - Status pulse animations
//! - Fade in/out effects

mod buttons;
mod indicators;

pub use buttons::{animated_button, animated_icon_button};
pub use indicators::{
    blocked_indicator, colored_spinner, pending_indicator, pulse_indicator, queued_indicator,
};

use eframe::egui::{self, Color32, Id, Pos2, Rect, Response, Sense, Ui, Vec2};

pub const HOVER_ANIM_SPEED: f64 = 8.0;
pub const FADE_ANIM_SPEED: f64 = 6.0;
pub const PULSE_SPEED: f64 = 3.0;
pub const SLIDE_ANIM_SPEED: f64 = 10.0;

/// Animated progress bar with smooth transitions
pub fn animated_progress_bar(
    ui: &mut Ui,
    progress: f32,
    color: Color32,
    id_salt: impl std::hash::Hash,
) {
    let id = Id::new(id_salt);

    let animated_progress = ui.ctx().animate_value_with_time(id, progress, 0.3);

    let desired_size = Vec2::new(ui.available_width(), 4.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    ui.painter()
        .rect_filled(rect, 2.0, Color32::from_rgb(40, 45, 55));

    let progress_rect = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * animated_progress, rect.height()),
    );
    ui.painter().rect_filled(progress_rect, 2.0, color);

    if animated_progress > 0.0 && animated_progress < 1.0 {
        let time = ui.ctx().input(|i| i.time);
        let shine_pos = ((time * 2.0) % 1.0) as f32;
        let shine_x = progress_rect.min.x + progress_rect.width() * shine_pos;

        if shine_x < progress_rect.max.x {
            let shine_color = Color32::from_rgba_unmultiplied(255, 255, 255, 60);
            ui.painter().rect_filled(
                Rect::from_center_size(
                    Pos2::new(shine_x, rect.center().y),
                    Vec2::new(20.0, rect.height()),
                ),
                2.0,
                shine_color,
            );
        }
        ui.ctx().request_repaint();
    }
}

/// Fade container - wraps content with fade animation
pub fn fade_in(ui: &mut Ui, id_salt: impl std::hash::Hash, visible: bool) -> f32 {
    let id = Id::new(id_salt);
    ui.ctx().animate_bool_with_time(id, visible, 0.2)
}

/// Get slide offset for panel transitions
pub fn slide_offset(ui: &mut Ui, id_salt: impl std::hash::Hash, target: f32) -> f32 {
    let id = Id::new(id_salt);
    ui.ctx().animate_value_with_time(id, target, 0.25)
}

/// Animated list item hover effect
pub fn list_item_background(
    ui: &mut Ui,
    rect: Rect,
    is_selected: bool,
    is_hovered: bool,
    id_salt: impl std::hash::Hash,
    selected_color: Color32,
    hover_color: Color32,
) {
    let id = Id::new(id_salt);

    let select_anim = ui
        .ctx()
        .animate_bool_with_time(id.with("select"), is_selected, 0.15);
    let hover_anim =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), is_hovered && !is_selected, 0.1);

    let bg_color = if select_anim > 0.01 {
        let mut c = selected_color;
        c = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (select_anim * 255.0) as u8);
        c
    } else if hover_anim > 0.01 {
        let mut c = hover_color;
        c = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (hover_anim * 180.0) as u8);
        c
    } else {
        Color32::TRANSPARENT
    };

    if bg_color.a() > 0 {
        ui.painter().rect_filled(rect, 4.0, bg_color);
    }
}

/// Animated clickable list item with hover effect
/// Returns (response, is_hovered) - response.clicked() can be used to detect clicks
pub fn animated_list_item<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    base_fill: Color32,
    hover_fill: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (Response, bool) {
    let id = Id::new(id_salt);

    let was_hovered = ui
        .ctx()
        .memory(|mem| mem.data.get_temp::<bool>(id).unwrap_or(false));

    let hover_anim = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), was_hovered, 0.12);

    let fill = lerp_color(base_fill, hover_fill, hover_anim);

    let frame_response = egui::Frame::NONE
        .fill(fill)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .stroke(egui::Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, (hover_anim * 20.0) as u8),
        ))
        .show(ui, |ui| add_contents(ui));

    let response = frame_response.response.interact(egui::Sense::click());
    let is_hovered = response.hovered();

    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    if is_hovered != was_hovered || hover_anim > 0.01 && hover_anim < 0.99 {
        ui.ctx().request_repaint();
    }

    (response, is_hovered)
}

/// Linear interpolation between two colors
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
        lerp_u8(a.a(), b.a(), t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

/// Typewriter effect for text (reveals characters over time)
pub struct TypewriterState {
    pub visible_chars: usize,
    pub last_update: f64,
}

impl TypewriterState {
    pub fn new() -> Self {
        Self {
            visible_chars: 0,
            last_update: 0.0,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, total_chars: usize, chars_per_second: f32) {
        let time = ctx.input(|i| i.time);
        let delta = time - self.last_update;

        if delta > 1.0 / chars_per_second as f64 {
            if self.visible_chars < total_chars {
                self.visible_chars += 1;
                self.last_update = time;
                ctx.request_repaint();
            }
        }
    }

    pub fn get_visible_text<'a>(&self, text: &'a str) -> &'a str {
        let byte_index = text
            .char_indices()
            .nth(self.visible_chars)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        &text[..byte_index]
    }
}

impl Default for TypewriterState {
    fn default() -> Self {
        Self::new()
    }
}
