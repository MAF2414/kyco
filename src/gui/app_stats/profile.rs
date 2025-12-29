//! Player profile section for the dashboard
//!
//! Shows level, XP progress, and streaks.

use eframe::egui::{self, RichText, Vec2};

use crate::gui::app::KycoApp;
use crate::gui::theme::{
    ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_HIGHLIGHT, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::stats::{PlayerStats, Streaks};

impl KycoApp {
    /// Render the player profile section at the top of the dashboard
    pub(super) fn render_profile_section(&mut self, ui: &mut egui::Ui) {
        // Load data if not cached
        self.ensure_gamification_data_loaded();

        let Some(player_stats) = &self.player_stats else {
            return;
        };
        let Some(streaks) = &self.streaks else {
            return;
        };

        // Clone data to avoid borrow issues
        let player_stats = player_stats.clone();
        let streaks = streaks.clone();

        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left: Level and XP
                    ui.vertical(|ui| {
                        render_level_display(ui, &player_stats);
                    });

                    ui.add_space(32.0);
                    ui.separator();
                    ui.add_space(32.0);

                    // Right: Streaks
                    ui.vertical(|ui| {
                        render_streaks_display(ui, &streaks);
                    });
                });
            });
    }

    /// Ensure gamification data is loaded from database
    fn ensure_gamification_data_loaded(&mut self) {
        if self.player_stats.is_none() || self.streaks.is_none() {
            if let Some(manager) = &self.stats_manager {
                if self.player_stats.is_none() {
                    self.player_stats = manager.achievements().get_player_stats().ok();
                }
                if self.streaks.is_none() {
                    self.streaks = manager.achievements().get_streaks().ok();
                }
            }
        }
    }

}

/// Render level and XP progress
fn render_level_display(ui: &mut egui::Ui, stats: &PlayerStats) {
    ui.label(RichText::new("PROFILE").small().color(TEXT_MUTED));
    ui.add_space(4.0);

    // Level and title
    ui.horizontal(|ui| {
        ui.label(RichText::new("👤").size(24.0));
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&stats.title)
                    .size(16.0)
                    .color(ACCENT_PURPLE)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("Level {}", stats.level))
                    .size(12.0)
                    .color(TEXT_DIM),
            );
        });
    });

    ui.add_space(8.0);

    // XP progress bar
    let progress = stats.progress_to_next();
    let bar_width = 150.0;
    let bar_height = 12.0;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(bar_width, bar_height), egui::Sense::hover());

    // Background
    ui.painter().rect_filled(rect, 4, BG_HIGHLIGHT);

    // Progress fill
    let fill_width = rect.width() * progress;
    let fill_rect = egui::Rect::from_min_size(rect.min, Vec2::new(fill_width, bar_height));
    ui.painter().rect_filled(fill_rect, 4, ACCENT_GREEN);

    // XP text
    let xp_text = if let Some(next) = stats.next_level_xp {
        format!("{} / {} XP", stats.total_xp, next)
    } else {
        format!("{} XP (MAX)", stats.total_xp)
    };
    ui.label(RichText::new(xp_text).small().color(TEXT_DIM));
}

/// Render streak information
fn render_streaks_display(ui: &mut egui::Ui, streaks: &Streaks) {
    ui.label(RichText::new("STREAKS").small().color(TEXT_MUTED));
    ui.add_space(4.0);

    // Daily streak
    ui.horizontal(|ui| {
        let daily_active = streaks.daily.is_active();
        let color = if daily_active { ACCENT_YELLOW } else { TEXT_DIM };

        ui.label(RichText::new("🔥").size(18.0));
        ui.label(RichText::new("Daily:").small().color(TEXT_MUTED));
        ui.label(
            RichText::new(format!("{} days", streaks.daily.current))
                .color(color)
                .strong(),
        );
        if streaks.daily.best > 0 && streaks.daily.best > streaks.daily.current {
            ui.label(
                RichText::new(format!("(best: {})", streaks.daily.best))
                    .small()
                    .color(TEXT_DIM),
            );
        }
    });

    ui.add_space(4.0);

    // Success streak
    ui.horizontal(|ui| {
        let success_color = if streaks.success.current >= 5 {
            ACCENT_GREEN
        } else {
            TEXT_PRIMARY
        };

        ui.label(RichText::new("✨").size(18.0));
        ui.label(RichText::new("Success:").small().color(TEXT_MUTED));
        ui.label(
            RichText::new(format!("{} jobs", streaks.success.current))
                .color(success_color)
                .strong(),
        );
        if streaks.success.best > 0 && streaks.success.best > streaks.success.current {
            ui.label(
                RichText::new(format!("(best: {})", streaks.success.best))
                    .small()
                    .color(TEXT_DIM),
            );
        }
    });
}
