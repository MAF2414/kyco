//! Achievements gallery view for KycoApp
//!
//! Displays all achievements grouped by category with unlock status.

use eframe::egui::{self, RichText, ScrollArea, Vec2};

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_YELLOW, BG_HIGHLIGHT, BG_PRIMARY, BG_SECONDARY,
    TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::stats::{Achievement, AchievementCategory, AchievementId};

impl KycoApp {
    /// Render the achievements gallery view
    pub(crate) fn render_achievements(&mut self, ctx: &egui::Context) {
        // Ensure gamification data is loaded
        self.ensure_achievements_loaded();

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_achievements_header(ui);
                        ui.add_space(16.0);
                        self.render_achievements_grid(ui);
                    });
            });
    }

    /// Ensure achievements data is loaded
    fn ensure_achievements_loaded(&mut self) {
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

    /// Render the achievements header with profile summary
    fn render_achievements_header(&mut self, ui: &mut egui::Ui) {
        let unlocked_ids = self
            .stats_manager
            .as_ref()
            .and_then(|m| m.achievements().get_unlocked_ids().ok())
            .unwrap_or_default();

        let unlocked_count = unlocked_ids.len();
        let total_count = Achievement::total_count();
        let total_xp: u32 = unlocked_ids
            .iter()
            .filter_map(|id| AchievementId::from_str(id))
            .map(|id| Achievement::get(id).xp_reward)
            .sum();

        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Back button
                    if ui
                        .button(RichText::new("< Stats").color(TEXT_MUTED))
                        .clicked()
                    {
                        self.view_mode = ViewMode::Stats;
                    }

                    ui.add_space(24.0);

                    // Title
                    ui.label(RichText::new("ACHIEVEMENTS").size(20.0).strong().color(ACCENT_YELLOW));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Progress
                        ui.label(
                            RichText::new(format!("{} / {}", unlocked_count, total_count))
                                .size(18.0)
                                .color(ACCENT_CYAN)
                                .strong(),
                        );
                        ui.label(RichText::new("unlocked").small().color(TEXT_MUTED));
                        ui.add_space(16.0);

                        // XP earned
                        ui.label(
                            RichText::new(format!("{} XP", total_xp))
                                .size(14.0)
                                .color(ACCENT_GREEN),
                        );
                        ui.label(RichText::new("earned").small().color(TEXT_MUTED));
                    });
                });
            });
    }

    /// Render the achievements grid grouped by category
    fn render_achievements_grid(&self, ui: &mut egui::Ui) {
        let unlocked_ids = self
            .stats_manager
            .as_ref()
            .and_then(|m| m.achievements().get_unlocked_ids().ok())
            .unwrap_or_default();

        // Categories to display (in order)
        let categories = [
            AchievementCategory::Milestone,
            AchievementCategory::Chain,
            AchievementCategory::Mode,
            AchievementCategory::Agent,
            AchievementCategory::Skill,
            AchievementCategory::Time,
            AchievementCategory::Streak,
            AchievementCategory::Token,
            AchievementCategory::Files,
            AchievementCategory::Tools,
            AchievementCategory::Cost,
            AchievementCategory::Lines,
            AchievementCategory::Duration,
            AchievementCategory::Special,
            AchievementCategory::Loyalty,
            AchievementCategory::Hidden,
            AchievementCategory::Whisper,
        ];

        for category in categories {
            let achievements = Achievement::by_category(category);
            if achievements.is_empty() {
                continue;
            }

            // Count unlocked in this category
            let unlocked_in_cat = achievements
                .iter()
                .filter(|a| unlocked_ids.contains(&a.id.as_str().to_string()))
                .count();

            // Category header
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(category.icon()).size(18.0));
                ui.label(
                    RichText::new(category.label())
                        .size(14.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(format!("({}/{})", unlocked_in_cat, achievements.len()))
                        .small()
                        .color(TEXT_DIM),
                );
            });
            ui.add_space(4.0);

            // Achievements in this category - use full width for proper wrapping
            let available_width = ui.available_width();
            egui::Frame::NONE
                .fill(BG_SECONDARY)
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_width(available_width - 16.0); // Account for margins
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                    ui.horizontal_wrapped(|ui| {
                        for achievement in achievements {
                            let is_unlocked =
                                unlocked_ids.contains(&achievement.id.as_str().to_string());

                            // For hidden achievements, only show if unlocked
                            if category.is_secret() && !is_unlocked {
                                render_locked_secret(ui);
                            } else {
                                render_achievement_badge(ui, achievement, is_unlocked);
                            }
                        }
                    });
                });
        }
    }
}

/// Render a single achievement badge
fn render_achievement_badge(ui: &mut egui::Ui, achievement: &Achievement, is_unlocked: bool) {
    let (bg_color, text_alpha) = if is_unlocked {
        (BG_HIGHLIGHT, 1.0)
    } else {
        (BG_PRIMARY, 0.4)
    };

    let frame_response = egui::Frame::NONE
        .fill(bg_color)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(100.0, 70.0));
            ui.set_max_width(110.0);

            ui.vertical_centered(|ui| {
                // Icon
                let icon_alpha = if is_unlocked { 1.0 } else { 0.3 };
                ui.label(
                    RichText::new(achievement.icon)
                        .size(24.0)
                        .color(egui::Color32::WHITE.gamma_multiply(icon_alpha)),
                );

                // Name
                let name_color = if is_unlocked { ACCENT_YELLOW } else { TEXT_DIM };
                ui.label(
                    RichText::new(truncate_name(achievement.name, 12))
                        .small()
                        .color(name_color.gamma_multiply(text_alpha)),
                );

                // XP reward
                if is_unlocked {
                    ui.label(
                        RichText::new(format!("+{} XP", achievement.xp_reward))
                            .small()
                            .color(ACCENT_GREEN),
                    );
                } else {
                    ui.label(RichText::new("Locked").small().color(TEXT_DIM));
                }
            });
        });

    // Hover tooltip with full info
    let tooltip_text = if is_unlocked {
        format!(
            "{}\n\n{}\n\n+{} XP",
            achievement.name, achievement.description, achievement.xp_reward
        )
    } else {
        format!(
            "{}\n\n{}\n\nUnlock to earn {} XP",
            achievement.name, achievement.description, achievement.xp_reward
        )
    };

    frame_response.response.on_hover_text(tooltip_text);
}

/// Render a locked secret achievement placeholder
fn render_locked_secret(ui: &mut egui::Ui) {
    let frame_response = egui::Frame::NONE
        .fill(BG_PRIMARY)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(100.0, 70.0));
            ui.set_max_width(110.0);

            ui.vertical_centered(|ui| {
                ui.label(RichText::new("?").size(24.0).color(TEXT_DIM));
                ui.label(RichText::new("???").small().color(TEXT_DIM));
                ui.label(RichText::new("Hidden").small().color(TEXT_DIM));
            });
        });

    frame_response
        .response
        .on_hover_text("A secret achievement. Keep playing to discover it!");
}

/// Truncate a name to fit in the badge
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len - 3])
    }
}
