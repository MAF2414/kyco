//! Toast notification system for gamification events
//!
//! Displays achievement unlocks, level-ups, and streak updates as temporary notifications.

use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, Id, RichText, Vec2};

use crate::gui::app::KycoApp;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_SECONDARY};
use crate::stats::{CompletedChallenge, GamificationEvent, LevelUp, UnlockedAchievement};

/// How long a toast is displayed
const TOAST_DURATION: Duration = Duration::from_secs(4);

/// Animation duration for fade in/out
const FADE_DURATION: f32 = 0.3;

impl KycoApp {
    /// Render toast notifications for gamification events
    pub(crate) fn render_toast(&mut self, ctx: &egui::Context) {
        // Check if we need to show a new toast
        if self.current_toast.is_none() {
            if let Some(event) = self.gamification_events.pop_front() {
                self.current_toast = Some((event, Instant::now()));
            }
        }

        // Render current toast if any
        let Some((event, start_time)) = &self.current_toast else {
            return;
        };

        let elapsed = start_time.elapsed();

        // Check if toast should be dismissed
        if elapsed > TOAST_DURATION {
            self.current_toast = None;
            ctx.request_repaint(); // Check for next toast
            return;
        }

        // Calculate fade alpha
        let progress = elapsed.as_secs_f32();
        let alpha = if progress < FADE_DURATION {
            // Fade in
            progress / FADE_DURATION
        } else if progress > TOAST_DURATION.as_secs_f32() - FADE_DURATION {
            // Fade out
            (TOAST_DURATION.as_secs_f32() - progress) / FADE_DURATION
        } else {
            1.0
        };

        // Animate the fade
        let animated_alpha = ctx.animate_value_with_time(
            Id::new("toast_alpha"),
            alpha,
            0.1,
        );

        // Clone event data for rendering
        let event = event.clone();

        // Render toast window
        egui::Area::new(Id::new("gamification_toast"))
            .anchor(Align2::RIGHT_TOP, Vec2::new(-20.0, 60.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let bg_color = Color32::from_rgba_unmultiplied(
                    BG_SECONDARY.r(),
                    BG_SECONDARY.g(),
                    BG_SECONDARY.b(),
                    (animated_alpha * 240.0) as u8,
                );

                egui::Frame::NONE
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(100, 100, 100, (animated_alpha * 150.0) as u8),
                    ))
                    .corner_radius(8.0)
                    .inner_margin(16.0)
                    .shadow(egui::Shadow {
                        spread: 4,
                        blur: 8,
                        color: Color32::from_rgba_unmultiplied(0, 0, 0, (animated_alpha * 100.0) as u8),
                        offset: [0, 2],
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(280.0);
                        render_toast_content(ui, &event, animated_alpha);
                    });
            });

        // Keep repainting for animation
        ctx.request_repaint();
    }
}

/// Render the content of a toast based on event type
fn render_toast_content(ui: &mut egui::Ui, event: &GamificationEvent, alpha: f32) {
    match event {
        GamificationEvent::AchievementUnlocked(unlocked) => {
            render_achievement_toast(ui, unlocked, alpha);
        }
        GamificationEvent::ChallengeCompleted(completed) => {
            render_challenge_toast(ui, completed, alpha);
        }
        GamificationEvent::LevelUp(level_up) => {
            render_level_up_toast(ui, level_up, alpha);
        }
        GamificationEvent::StreakExtended { streak_type, count } => {
            render_streak_toast(ui, streak_type, *count, alpha);
        }
        GamificationEvent::XpAwarded { amount, reason } => {
            render_xp_toast(ui, *amount, reason, alpha);
        }
    }
}

/// Render achievement unlock toast
fn render_achievement_toast(ui: &mut egui::Ui, unlocked: &UnlockedAchievement, alpha: f32) {
    let achievement = unlocked.achievement;
    let color = apply_alpha(ACCENT_YELLOW, alpha);

    ui.horizontal(|ui| {
        // Icon (large)
        ui.label(RichText::new(achievement.icon).size(32.0));

        ui.vertical(|ui| {
            ui.label(
                RichText::new("Achievement Unlocked!")
                    .color(color)
                    .size(12.0),
            );
            ui.label(
                RichText::new(achievement.name)
                    .color(apply_alpha(Color32::WHITE, alpha))
                    .strong()
                    .size(16.0),
            );
            ui.label(
                RichText::new(achievement.description)
                    .color(apply_alpha(Color32::GRAY, alpha))
                    .size(11.0),
            );
            ui.label(
                RichText::new(format!("+{} XP", achievement.xp_reward))
                    .color(apply_alpha(ACCENT_GREEN, alpha))
                    .size(12.0),
            );
        });
    });
}

/// Render challenge completed toast
fn render_challenge_toast(ui: &mut egui::Ui, completed: &CompletedChallenge, alpha: f32) {
    let challenge = completed.challenge;
    let color = apply_alpha(ACCENT_CYAN, alpha);

    ui.horizontal(|ui| {
        // Icon (large)
        ui.label(RichText::new(challenge.icon).size(32.0));

        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("Challenge #{} Complete!", challenge.id.number()))
                    .color(color)
                    .size(12.0),
            );
            ui.label(
                RichText::new(challenge.name)
                    .color(apply_alpha(Color32::WHITE, alpha))
                    .strong()
                    .size(16.0),
            );
            ui.label(
                RichText::new(challenge.description)
                    .color(apply_alpha(Color32::GRAY, alpha))
                    .size(11.0),
            );
            ui.label(
                RichText::new(format!("+{} XP", challenge.xp_reward))
                    .color(apply_alpha(ACCENT_GREEN, alpha))
                    .size(12.0),
            );
        });
    });
}

/// Render level up toast
fn render_level_up_toast(ui: &mut egui::Ui, level_up: &LevelUp, alpha: f32) {
    let color = apply_alpha(ACCENT_PURPLE, alpha);

    ui.horizontal(|ui| {
        ui.label(RichText::new("🎉").size(32.0));

        ui.vertical(|ui| {
            ui.label(RichText::new("LEVEL UP!").color(color).strong().size(14.0));
            ui.label(
                RichText::new(format!("Level {} → {}", level_up.old_level, level_up.new_level))
                    .color(apply_alpha(Color32::WHITE, alpha))
                    .size(18.0)
                    .strong(),
            );
            ui.label(
                RichText::new(&level_up.new_title)
                    .color(apply_alpha(ACCENT_CYAN, alpha))
                    .size(14.0),
            );
        });
    });
}

/// Render streak extended toast
fn render_streak_toast(
    ui: &mut egui::Ui,
    streak_type: &crate::stats::StreakType,
    count: u32,
    alpha: f32,
) {
    let (icon, label) = match streak_type {
        crate::stats::StreakType::Daily => ("🔥", "Daily Streak"),
        crate::stats::StreakType::Success => ("✨", "Success Streak"),
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(28.0));

        ui.vertical(|ui| {
            ui.label(
                RichText::new(label)
                    .color(apply_alpha(ACCENT_YELLOW, alpha))
                    .size(12.0),
            );
            ui.label(
                RichText::new(format!("{} days!", count))
                    .color(apply_alpha(Color32::WHITE, alpha))
                    .strong()
                    .size(18.0),
            );
        });
    });
}

/// Render XP awarded toast (only shown for significant amounts)
fn render_xp_toast(ui: &mut egui::Ui, amount: u32, _reason: &str, alpha: f32) {
    // Only show for larger XP gains (achievements, etc.)
    if amount < 20 {
        return;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("⭐").size(24.0));
        ui.label(
            RichText::new(format!("+{} XP", amount))
                .color(apply_alpha(ACCENT_GREEN, alpha))
                .strong()
                .size(16.0),
        );
    });
}

/// Apply alpha to a color
fn apply_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * alpha) as u8,
    )
}
