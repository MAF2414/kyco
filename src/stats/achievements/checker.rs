//! Achievement checking logic
//!
//! Contains functions to check if achievements should be unlocked.
//! Supports 129 achievements across 17 categories (including Hidden, Whisper, and Loyalty).

use super::definitions::AchievementId;
use super::streaks::{current_hour, is_weekend};
use crate::stats::models::JobStatsRecord;
use chrono::{Datelike, Local, Timelike};

/// Check milestone achievements based on job counts (10 achievements)
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
        (250, AchievementId::TwoFifty),
        (500, AchievementId::FiveHundred),
        (1000, AchievementId::Thousand),
        (2500, AchievementId::TwoThousandFive),
        (5000, AchievementId::FiveThousand),
        (10000, AchievementId::TenThousand),
    ];

    for (threshold, id) in milestones {
        if total_jobs >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check chain achievements (6 achievements)
pub fn check_chain_achievements(
    total_chains: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (1, AchievementId::FirstChain),
        (5, AchievementId::FiveChains),
        (10, AchievementId::TenChains),
        (25, AchievementId::TwentyFiveChains),
        (50, AchievementId::FiftyChains),
        (100, AchievementId::HundredChains),
    ];

    for (threshold, id) in milestones {
        if total_chains >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check mode diversity achievements (5 achievements)
pub fn check_mode_achievements(
    unique_modes: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (3, AchievementId::ModeNovice),
        (5, AchievementId::Polyglot),
        (10, AchievementId::ModeExplorer),
        (15, AchievementId::ModeMaster),
        (20, AchievementId::ModeCollector),
    ];

    for (threshold, id) in milestones {
        if unique_modes >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check agent diversity achievements (4 achievements)
pub fn check_agent_achievements(
    unique_agents: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (2, AchievementId::MultiAgent),
        (3, AchievementId::TripleAgent),
        (4, AchievementId::QuadAgent),
        (5, AchievementId::AgentCollector),
    ];

    for (threshold, id) in milestones {
        if unique_agents >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check skill-based achievements for a specific job (10 achievements)
pub fn check_skill_achievements(
    job: &JobStatsRecord,
    success_streak: u32,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();
    let job_succeeded = job.status == "done" || job.status == "merged";

    // Flawless streaks (5 achievements)
    let flawless_milestones = [
        (5, AchievementId::Flawless5),
        (10, AchievementId::Flawless10),
        (25, AchievementId::Flawless25),
        (50, AchievementId::Flawless50),
        (100, AchievementId::Flawless100),
    ];

    for (threshold, id) in flawless_milestones {
        if success_streak >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    // Speed achievements (only for successful jobs)
    if job_succeeded {
        // Speed Demon: Job completed in under 30 seconds
        if job.duration_ms < 30_000
            && !unlocked.contains(&AchievementId::SpeedDemon.as_str().to_string())
        {
            newly_unlocked.push(AchievementId::SpeedDemon);
        }

        // Lightning Fast: Job completed in under 15 seconds
        if job.duration_ms < 15_000
            && !unlocked.contains(&AchievementId::LightningFast.as_str().to_string())
        {
            newly_unlocked.push(AchievementId::LightningFast);
        }

        // Instant: Job completed in under 10 seconds
        if job.duration_ms < 10_000
            && !unlocked.contains(&AchievementId::Instant.as_str().to_string())
        {
            newly_unlocked.push(AchievementId::Instant);
        }

        // Token efficiency achievements
        let total_tokens = job.input_tokens + job.output_tokens;

        // Token Saver: Job with < 500 tokens
        if total_tokens < 500
            && !unlocked.contains(&AchievementId::TokenSaver.as_str().to_string())
        {
            newly_unlocked.push(AchievementId::TokenSaver);
        }

        // Efficient: Job with < 1000 tokens
        if total_tokens < 1000
            && !unlocked.contains(&AchievementId::Efficient.as_str().to_string())
        {
            newly_unlocked.push(AchievementId::Efficient);
        }
    }

    newly_unlocked
}

/// Check time-based achievements (5 achievements)
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

    // Lunch Coder: 12 PM to 1 PM
    if hour == 12 && !unlocked.contains(&AchievementId::LunchCoder.as_str().to_string()) {
        newly_unlocked.push(AchievementId::LunchCoder);
    }

    // Late Night: 10 PM to midnight
    if (22..24).contains(&hour) && !unlocked.contains(&AchievementId::LateNight.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::LateNight);
    }

    newly_unlocked
}

/// Check streak-based achievements (10 achievements)
pub fn check_streak_achievements(
    daily_streak: u32,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (3, AchievementId::Streak3),
        (7, AchievementId::Streak7),
        (14, AchievementId::Streak14),
        (30, AchievementId::Streak30),
        (60, AchievementId::Streak60),
        (90, AchievementId::Streak90),
        (180, AchievementId::Streak180),
        (365, AchievementId::Streak365),
    ];

    for (threshold, id) in milestones {
        if daily_streak >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check success streak achievements (separate from daily streak)
pub fn check_success_streak_achievements(
    success_streak: u32,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    if success_streak >= 10 && !unlocked.contains(&AchievementId::SuccessStreak10.as_str().to_string()) {
        newly_unlocked.push(AchievementId::SuccessStreak10);
    }
    if success_streak >= 25 && !unlocked.contains(&AchievementId::SuccessStreak25.as_str().to_string()) {
        newly_unlocked.push(AchievementId::SuccessStreak25);
    }

    newly_unlocked
}

/// Check token accumulation achievements (15 achievements)
pub fn check_token_achievements(
    total_tokens: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones: [(u64, AchievementId); 15] = [
        (10_000, AchievementId::Tokens10k),
        (50_000, AchievementId::Tokens50k),
        (100_000, AchievementId::Tokens100k),
        (500_000, AchievementId::Tokens500k),
        (1_000_000, AchievementId::Tokens1m),
        (5_000_000, AchievementId::Tokens5m),
        (10_000_000, AchievementId::Tokens10m),
        (50_000_000, AchievementId::Tokens50m),
        (100_000_000, AchievementId::Tokens100m),
        (500_000_000, AchievementId::Tokens500m),
        (1_000_000_000, AchievementId::Tokens1b),
        (10_000_000_000, AchievementId::Tokens10b),
        (100_000_000_000, AchievementId::Tokens100b),
        (500_000_000_000, AchievementId::Tokens500b),
        (1_000_000_000_000, AchievementId::Tokens1t),
    ];

    for (threshold, id) in milestones {
        if total_tokens >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check file access achievements (16 achievements)
pub fn check_file_achievements(
    total_file_accesses: u64,
    unique_files: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    // Total file accesses
    let access_milestones = [
        (10, AchievementId::Files10),
        (50, AchievementId::Files50),
        (100, AchievementId::Files100),
        (500, AchievementId::Files500),
        (1000, AchievementId::Files1k),
        (5000, AchievementId::Files5k),
        (10000, AchievementId::Files10k),
        (50000, AchievementId::Files50k),
        (100000, AchievementId::Files100k),
        (500000, AchievementId::Files500k),
        (1000000, AchievementId::Files1m),
    ];

    for (threshold, id) in access_milestones {
        if total_file_accesses >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    // Unique files
    let unique_milestones = [
        (50, AchievementId::UniqueFiles50),
        (100, AchievementId::UniqueFiles100),
        (500, AchievementId::UniqueFiles500),
        (1000, AchievementId::UniqueFiles1k),
        (5000, AchievementId::UniqueFiles5k),
    ];

    for (threshold, id) in unique_milestones {
        if unique_files >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check tool usage achievements (16 achievements)
pub fn check_tool_achievements(
    total_tool_calls: u64,
    unique_tools: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    // Total tool calls
    let call_milestones = [
        (100, AchievementId::ToolCalls100),
        (500, AchievementId::ToolCalls500),
        (1000, AchievementId::ToolCalls1k),
        (5000, AchievementId::ToolCalls5k),
        (10000, AchievementId::ToolCalls10k),
        (50000, AchievementId::ToolCalls50k),
        (100000, AchievementId::ToolCalls100k),
        (500000, AchievementId::ToolCalls500k),
        (1000000, AchievementId::ToolCalls1m),
        (5000000, AchievementId::ToolCalls5m),
        (10000000, AchievementId::ToolCalls10m),
    ];

    for (threshold, id) in call_milestones {
        if total_tool_calls >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    // Unique tools
    let unique_milestones = [
        (5, AchievementId::UniqueTools5),
        (10, AchievementId::UniqueTools10),
        (20, AchievementId::UniqueTools20),
        (50, AchievementId::UniqueTools50),
        (100, AchievementId::UniqueTools100),
    ];

    for (threshold, id) in unique_milestones {
        if unique_tools >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check cost/spending achievements (12 achievements)
pub fn check_cost_achievements(
    total_cost_usd: f64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (1.0, AchievementId::Spent1),
        (10.0, AchievementId::Spent10),
        (50.0, AchievementId::Spent50),
        (100.0, AchievementId::Spent100),
        (500.0, AchievementId::Spent500),
        (1000.0, AchievementId::Spent1000),
        (2500.0, AchievementId::Spent2500),
        (5000.0, AchievementId::Spent5000),
        (10000.0, AchievementId::Spent10000),
        (25000.0, AchievementId::Spent25000),
        (50000.0, AchievementId::Spent50000),
        (100000.0, AchievementId::Spent100000),
    ];

    for (threshold, id) in milestones {
        if total_cost_usd >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check lines of code achievements (12 achievements)
pub fn check_lines_achievements(
    total_lines_changed: u64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (100, AchievementId::Lines100),
        (500, AchievementId::Lines500),
        (1000, AchievementId::Lines1k),
        (5000, AchievementId::Lines5k),
        (10000, AchievementId::Lines10k),
        (50000, AchievementId::Lines50k),
        (100000, AchievementId::Lines100k),
        (500000, AchievementId::Lines500k),
        (1000000, AchievementId::Lines1m),
        (5000000, AchievementId::Lines5m),
        (10000000, AchievementId::Lines10m),
        (50000000, AchievementId::Lines50m),
    ];

    for (threshold, id) in milestones {
        if total_lines_changed >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check duration/time spent achievements (8 achievements)
pub fn check_duration_achievements(
    total_duration_hours: f64,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    let milestones = [
        (1.0, AchievementId::Duration1h),
        (10.0, AchievementId::Duration10h),
        (100.0, AchievementId::Duration100h),
        (1000.0, AchievementId::Duration1000h),
        (2500.0, AchievementId::Duration2500h),
        (5000.0, AchievementId::Duration5000h),
        (10000.0, AchievementId::Duration10000h),
        (25000.0, AchievementId::Duration25000h),
    ];

    for (threshold, id) in milestones {
        if total_duration_hours >= threshold && !unlocked.contains(&id.as_str().to_string()) {
            newly_unlocked.push(id);
        }
    }

    newly_unlocked
}

/// Check special achievements (11 achievements)
/// These require special conditions or specific contexts
pub fn check_special_achievements(
    ctx: &SpecialAchievementContext,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    // First of day achievements
    if ctx.first_of_day_count >= 10
        && !unlocked.contains(&AchievementId::FirstOfDay10.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::FirstOfDay10);
    }
    if ctx.first_of_day_count >= 50
        && !unlocked.contains(&AchievementId::FirstOfDay50.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::FirstOfDay50);
    }

    // Jobs in one day achievements
    if ctx.jobs_today >= 10 && !unlocked.contains(&AchievementId::Marathon.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Marathon);
    }
    if ctx.jobs_today >= 50 && !unlocked.contains(&AchievementId::Prolific.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Prolific);
    }
    if ctx.jobs_today >= 100 && !unlocked.contains(&AchievementId::Workhorse.as_str().to_string()) {
        newly_unlocked.push(AchievementId::Workhorse);
    }

    // Pair programmer: Claude + Codex in same day
    if ctx.used_claude_today && ctx.used_codex_today
        && !unlocked.contains(&AchievementId::PairProgrammer.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::PairProgrammer);
    }

    // New Year: Job on January 1st
    let now = Local::now();
    if now.month() == 1 && now.day() == 1
        && !unlocked.contains(&AchievementId::NewYear.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::NewYear);
    }

    // Midnight Oil: Job at exactly midnight (within first minute)
    let hour = now.hour();
    let minute = now.minute();
    if hour == 0 && minute == 0
        && !unlocked.contains(&AchievementId::MidnightOil.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::MidnightOil);
    }

    // Lucky Seven: 7 successful jobs 7 times (49 total in streaks of 7)
    if ctx.lucky_seven_progress >= 49
        && !unlocked.contains(&AchievementId::LuckySeven.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::LuckySeven);
    }

    // Dedication: 365 unique days
    if ctx.unique_days >= 365
        && !unlocked.contains(&AchievementId::Dedication.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::Dedication);
    }

    // 10K Club: 10,000 successful jobs total
    if ctx.total_successful_jobs >= 10000
        && !unlocked.contains(&AchievementId::TenKClub.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::TenKClub);
    }

    newly_unlocked
}

/// Context for special achievement checks
pub struct SpecialAchievementContext {
    pub first_of_day_count: u32,
    pub jobs_today: u32,
    pub used_claude_today: bool,
    pub used_codex_today: bool,
    pub lucky_seven_progress: u32,
    pub unique_days: u32,
    pub total_successful_jobs: u64,
}

impl Default for SpecialAchievementContext {
    fn default() -> Self {
        Self {
            first_of_day_count: 0,
            jobs_today: 0,
            used_claude_today: false,
            used_codex_today: false,
            lucky_seven_progress: 0,
            unique_days: 0,
            total_successful_jobs: 0,
        }
    }
}

/// Check loyalty achievements based on agent preference (4 achievements)
/// These track whether users prefer Claude, Codex, or use both equally
pub fn check_loyalty_achievements(
    ctx: &LoyaltyContext,
    unlocked: &[String],
) -> Vec<AchievementId> {
    let mut newly_unlocked = Vec::new();

    // DarioFan: 200 more Claude jobs than Codex
    if ctx.claude_jobs > ctx.codex_jobs
        && ctx.claude_jobs - ctx.codex_jobs >= 200
        && !unlocked.contains(&AchievementId::DarioFan.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::DarioFan);
    }

    // SamStan: 200 more Codex jobs than Claude
    if ctx.codex_jobs > ctx.claude_jobs
        && ctx.codex_jobs - ctx.claude_jobs >= 200
        && !unlocked.contains(&AchievementId::SamStan.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::SamStan);
    }

    // Switzerland: Exactly equal Claude and Codex jobs (50+ each)
    if ctx.claude_jobs >= 50
        && ctx.codex_jobs >= 50
        && ctx.claude_jobs == ctx.codex_jobs
        && !unlocked.contains(&AchievementId::Switzerland.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::Switzerland);
    }

    // Polygamous: 100+ jobs with each agent (Claude AND Codex both >= 100)
    if ctx.claude_jobs >= 100
        && ctx.codex_jobs >= 100
        && !unlocked.contains(&AchievementId::Polygamous.as_str().to_string())
    {
        newly_unlocked.push(AchievementId::Polygamous);
    }

    newly_unlocked
}

/// Context for loyalty achievement checks
pub struct LoyaltyContext {
    pub claude_jobs: u64,
    pub codex_jobs: u64,
}

impl Default for LoyaltyContext {
    fn default() -> Self {
        Self {
            claude_jobs: 0,
            codex_jobs: 0,
        }
    }
}
