//! Achievement checking logic
//!
//! Contains functions to check if achievements should be unlocked.

use super::definitions::AchievementId;
use super::streaks::{current_hour, is_weekend};
use crate::stats::models::JobStatsRecord;

/// Context for checking achievements after a job
pub struct CheckContext {
    pub total_jobs_done: u64,
    pub total_chains_done: u64,
    pub unique_modes: u64,
    pub unique_agents: u64,
    pub success_streak: u32,
    pub daily_streak: u32,
}

/// Check milestone achievements based on job counts
pub fn check_milestone_achievements(
    total_jobs: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (1, AchievementId::FirstJob),
        (10, AchievementId::TenJobs),
        (50, AchievementId::FiftyJobs),
        (100, AchievementId::Century),
        (500, AchievementId::FiveHundred),
        (1000, AchievementId::Thousand),
    ];

    for (threshold, id) in milestones {
        if total_jobs >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check chain achievements
pub fn check_chain_achievements(
    total_chains: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    if total_chains >= 1 && !unlocked.contains(&AchievementId::FirstChain.as_str().to_string()) {
        newly_unlocked.push(AchievementId::FirstChain);
    }
    if total_chains >= 10 && !unlocked.contains(&AchievementId::ChainMaster.as_str().to_string()) {
        newly_unlocked.push(AchievementId::ChainMaster);
    }

    newly_unlocked
}

/// Check mode diversity achievements
pub fn check_mode_achievements(
    unique_modes: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    if unique_modes >= 5 && !unlocked.contains(&AchievementId::Polyglot.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Polyglot);
    }
    if unique_modes >= 10 && !unlocked.contains(&AchievementId::ModeExplorer.as_str().to_string()) {
        newly_unlocked.push(AchievementId::ModeExplorer);
    }

    newly_unlocked
}

/// Check agent diversity achievements
pub fn check_agent_achievements(
    unique_agents: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    if unique_agents >= 2 && !unlocked.contains(&AchievementId::MultiAgent.as_str().to_string()) {
        newly_unlocked.push(AchievementId::MultiAgent);
    }

    newly_unlocked
}

/// Check skill-based achievements for a specific job
pub fn check_skill_achievements(
    job: &JobStatsRecord,
    success_streak: u32,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    // Speed Demon: Job completed in under 30 seconds
    if job.duration_ms < 30_000
        && job.status == "done"
        && !unlocked.contains(&AchievementId::SpeedDemon.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::SpeedDemon);
    }

    // Token Saver: Job with < 500 tokens
    let total_tokens = job.input_tokens + job.output_tokens;
    if total_tokens < 500
        && job.status == "done"
        && !unlocked.contains(&AchievementId::TokenSaver.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::TokenSaver);
    }

    // Flawless streaks
    if success_streak >= 10
        && !unlocked.contains(&AchievementId::Flawless10.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::Flawless10);
    }
    if success_streak >= 25
        && !unlocked.contains(&AchievementId::Flawless25.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::Flawless25);
    }

    newly_unlocked
}

/// Check time-based achievements
pub fn check_time_achievements(unlocked: &[String]) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();
    let hour = current_hour();

    // Night Owl: Midnight to 5 AM
    if hour < 5 && !unlocked.contains(&AchievementId::NightOwl.as_str().to_string()) {
        newly_unlocked.push(AchievementId::NightOwl);
    }

    // Early Bird: 5 AM to 7 AM
    if (5..7).contains(&hour) && !unlocked.contains(&AchievementId::EarlyBird.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::EarlyBird);
    }

    // Weekend Warrior
    if is_weekend() && !unlocked.contains(&AchievementId::WeekendWarrior.as_str().to_string()) {
        newly_unlocked.push(AchievementId::WeekendWarrior);
    }

    newly_unlocked
}

/// Check streak-based achievements
pub fn check_streak_achievements(
    daily_streak: u32,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    if daily_streak >= 3 && !unlocked.contains(&AchievementId::Streak3.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Streak3);
    }
    if daily_streak >= 7 && !unlocked.contains(&AchievementId::Streak7.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Streak7);
    }
    if daily_streak >= 30 && !unlocked.contains(&AchievementId::Streak30.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Streak30);
    }

    newly_unlocked
}
