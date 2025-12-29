//! Gamification system: Achievements, XP, Levels, Streaks, and Challenges
//!
//! This module provides a complete gamification layer on top of the stats system.

mod checker;
mod definitions;
mod levels;
mod manager;
mod streaks;

pub use definitions::{Achievement, AchievementCategory, AchievementId, ACHIEVEMENTS};
pub use levels::{Level, PlayerStats, LEVELS};
pub use manager::{AchievementManager, GamificationEvent, LevelUp, UnlockedAchievement};
pub use streaks::{StreakType, Streaks};
