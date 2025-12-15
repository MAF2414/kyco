//! Animation utilities for the GUI
//!
//! Provides smooth transitions and visual feedback through:
//! - Hover effects for interactive elements
//! - View transition animations
//! - Status pulse animations
//! - Fade in/out effects

use eframe::egui::{self, Color32, Id, Pos2, Rect, Response, RichText, Sense, Ui, Vec2};

// Animation timing constants
pub const HOVER_ANIM_SPEED: f64 = 8.0;      // Speed for hover transitions
pub const FADE_ANIM_SPEED: f64 = 6.0;       // Speed for fade in/out
pub const PULSE_SPEED: f64 = 3.0;           // Speed for pulse animations
pub const SLIDE_ANIM_SPEED: f64 = 10.0;     // Speed for slide transitions

/// Animated button with hover glow effect
pub fn animated_button(
    ui: &mut Ui,
    text: impl Into<RichText>,
    base_color: Color32,
    id_salt: impl std::hash::Hash,
) -> Response {
    let text = text.into();
    let id = Id::new(id_salt);

    // Read previous hover state from memory (defaults to false)
    let was_hovered = ui.ctx().memory(|mem| {
        mem.data.get_temp::<bool>(id).unwrap_or(false)
    });

    // Get smooth animation progress toward the previous hover state
    let hover_anim = ui.ctx().animate_bool_with_time(id.with("anim"), was_hovered, 0.15);

    // Calculate animated colors
    let glow_alpha = (hover_anim * 40.0) as u8;
    let fill_color = Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(),
        base_color.b(),
        glow_alpha,
    );
    let stroke_alpha = (hover_anim * 0.6 * 255.0) as u8;
    let stroke_color = Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(),
        base_color.b(),
        stroke_alpha,
    );

    // Create button with animated styling
    let button = egui::Button::new(text.color(base_color))
        .fill(fill_color)
        .stroke(egui::Stroke::new(1.0, stroke_color));

    let response = ui.add(button);

    // Store current hover state for next frame
    let is_hovered = response.hovered();
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    // Request repaint if animating
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

    // Read previous hover state from memory
    let was_hovered = ui.ctx().memory(|mem| {
        mem.data.get_temp::<bool>(id).unwrap_or(false)
    });

    // Animate hover state
    let hover_progress = ui.ctx().animate_bool_with_time(id.with("anim"), was_hovered, 0.12);
    let current_color = lerp_color(base_color, hover_color, hover_progress);

    let response = ui.add(
        egui::Button::new(RichText::new(icon).color(current_color))
            .fill(Color32::TRANSPARENT)
            .frame(false),
    );

    // Store current hover state for next frame
    let is_hovered = response.hovered();
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    // Request repaint if animating
    if is_hovered != was_hovered || hover_progress > 0.01 && hover_progress < 0.99 {
        ui.ctx().request_repaint();
    }

    response
}

/// Pulsing indicator for running/active states
pub fn pulse_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let pulse = ((time * PULSE_SPEED).sin() * 0.5 + 0.5) as f32;

    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());

    // Outer glow
    let glow_color = Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (pulse * 80.0) as u8,
    );
    ui.painter().circle_filled(rect.center(), size * 0.8, glow_color);

    // Inner solid
    let inner_alpha = 150 + (pulse * 105.0) as u8;
    let inner_color = Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        inner_alpha,
    );
    ui.painter().circle_filled(rect.center(), size * 0.4, inner_color);

    // Request continuous repaint for animation
    ui.ctx().request_repaint();
}

/// Animated progress bar with smooth transitions
pub fn animated_progress_bar(ui: &mut Ui, progress: f32, color: Color32, id_salt: impl std::hash::Hash) {
    let id = Id::new(id_salt);

    // Smooth the progress value
    let animated_progress = ui.ctx().animate_value_with_time(id, progress, 0.3);

    let desired_size = Vec2::new(ui.available_width(), 4.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    // Background track
    ui.painter().rect_filled(
        rect,
        2.0,
        Color32::from_rgb(40, 45, 55),
    );

    // Progress fill
    let progress_rect = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * animated_progress, rect.height()),
    );
    ui.painter().rect_filled(progress_rect, 2.0, color);

    // Animated shine effect when progressing
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

    // Animate selection and hover separately
    let select_anim = ui.ctx().animate_bool_with_time(
        id.with("select"),
        is_selected,
        0.15,
    );
    let hover_anim = ui.ctx().animate_bool_with_time(
        id.with("hover"),
        is_hovered && !is_selected,
        0.1,
    );

    // Blend colors based on animation state
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

/// Spinner with custom color (animated)
pub fn colored_spinner(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());

    let center = rect.center();
    let radius = size * 0.4;

    // Draw spinning arc
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

    // Read previous hover state
    let was_hovered = ui.ctx().memory(|mem| {
        mem.data.get_temp::<bool>(id).unwrap_or(false)
    });

    // Get animation progress
    let hover_anim = ui.ctx().animate_bool_with_time(id.with("hover"), was_hovered, 0.12);

    // Calculate animated fill color
    let fill = lerp_color(base_fill, hover_fill, hover_anim);

    // Render frame with animated background
    let frame_response = egui::Frame::NONE
        .fill(fill)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .stroke(egui::Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, (hover_anim * 20.0) as u8),
        ))
        .show(ui, |ui| {
            add_contents(ui)
        });

    // Make the frame interactive
    let response = frame_response.response.interact(egui::Sense::click());
    let is_hovered = response.hovered();

    // Store hover state
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp(id, is_hovered);
    });

    // Request repaint if animating
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
        let byte_index = text.char_indices()
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

/// Animated lock icon for blocked status (pulsing lock brackets)
pub fn blocked_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    // Slow pulse for "waiting" feel
    let pulse = ((time * 1.5).sin() * 0.5 + 0.5) as f32;
    let alpha = 140 + (pulse * 115.0) as u8;
    let animated_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    // Allocate space and render the lock symbol
    let text = RichText::new("[L]").monospace().color(animated_color).size(size);
    ui.label(text);

    ui.ctx().request_repaint();
}

/// Animated dots for queued status (cycling through . .. ...)
pub fn queued_indicator(ui: &mut Ui, color: Color32, size: f32) {
    let time = ui.ctx().input(|i| i.time);
    // Cycle through 4 states every ~1.2 seconds
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
    // Very gentle breathing animation
    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
    let alpha = 100 + (pulse * 155.0) as u8;
    let animated_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let text = RichText::new("[.]").monospace().color(animated_color).size(size);
    ui.label(text);

    ui.ctx().request_repaint();
}
