//! Gamification system: Achievements, XP, Levels, Streaks, and Challenges
//!
//! This module provides a complete gamification layer on top of the stats system.

mod challenges;
mod checker;
mod definitions;
mod levels;
mod manager;
mod streaks;

pub use challenges::{
    Challenge, ChallengeId, ChallengeProgress, ChallengeRequirement, ChallengeTier, CHALLENGES,
};
pub use definitions::{Achievement, AchievementCategory, AchievementId, ACHIEVEMENTS};
pub use levels::{
    level_for_xp, title_for_level, xp_for_level, LevelTier, LevelUp, PlayerStats, XpRewards,
    MAX_LEVEL,
};
pub use manager::{
    AchievementManager, ChallengeState, CompletedChallenge, GamificationEvent, UnlockedAchievement,
};
pub use streaks::{StreakType, Streaks};
