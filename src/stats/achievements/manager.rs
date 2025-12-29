//! Achievement Manager - Core gamification logic
//!
//! Handles achievement checking, XP awards, streak updates, challenge progress,
//! and database operations.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::Connection;

use super::challenges::{Challenge, ChallengeProgress, ChallengeRequirement};
use super::checker::{
    check_agent_achievements, check_chain_achievements, check_cost_achievements,
    check_duration_achievements, check_file_achievements, check_lines_achievements,
    check_loyalty_achievements, check_milestone_achievements, check_mode_achievements,
    check_skill_achievements, check_special_achievements, check_streak_achievements,
    check_success_streak_achievements, check_time_achievements, check_token_achievements,
    check_tool_achievements, LoyaltyContext, SpecialAchievementContext,
};
use super::definitions::{Achievement, AchievementId};
use super::levels::{level_for_xp, title_for_level, LevelUp, PlayerStats, XpRewards};
use super::streaks::{is_weekend, today_string, StreakInfo, StreakType, Streaks};
use crate::stats::models::JobStatsRecord;

/// An achievement that was just unlocked
#[derive(Debug, Clone)]
pub struct UnlockedAchievement {
    pub achievement: &'static Achievement,
    pub unlocked_at: i64,
}

/// A challenge that was just completed
#[derive(Debug, Clone)]
pub struct CompletedChallenge {
    pub challenge: &'static Challenge,
    pub completed_at: i64,
}

/// Events that can happen during gamification checks
#[derive(Debug, Clone)]
pub enum GamificationEvent {
    AchievementUnlocked(UnlockedAchievement),
    ChallengeCompleted(CompletedChallenge),
    LevelUp(LevelUp),
    StreakExtended { streak_type: StreakType, count: u32 },
    XpAwarded { amount: u32, reason: String },
}

/// Main manager for all gamification features
pub struct AchievementManager {
    conn: Arc<Mutex<Connection>>,
}

/// All stats needed for achievement checking
#[derive(Default)]
struct AchievementStats {
    total_jobs: u64,
    total_chains: u64,
    unique_modes: u64,
    unique_agents: u64,
    total_tokens: u64,
    total_file_accesses: u64,
    unique_files: u64,
    total_tool_calls: u64,
    unique_tools: u64,
    total_cost_usd: f64,
    total_lines_changed: u64,
    total_duration_hours: f64,
    jobs_today: u32,
    first_of_day_count: u32,
    unique_days: u32,
    used_claude_today: bool,
    used_codex_today: bool,
    // Loyalty stats for Claude vs Codex preference tracking
    claude_jobs: u64,
    codex_jobs: u64,
}

impl AchievementManager {
    /// Create a new manager with a database connection
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get current timestamp in milliseconds
    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    // ========================================
    // ACHIEVEMENT OPERATIONS
    // ========================================

    /// Get all unlocked achievement IDs
    pub fn get_unlocked_ids(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().expect("lock");
        let mut stmt = conn.prepare("SELECT id FROM achievements")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    /// Unlock an achievement
    fn unlock_achievement(&self, id: AchievementId) -> Result<UnlockedAchievement> {
        let now = Self::now_ms();
        let conn = self.conn.lock().expect("lock");
        conn.execute(
            "INSERT OR IGNORE INTO achievements (id, unlocked_at) VALUES (?1, ?2)",
            (id.as_str(), now),
        )?;
        drop(conn);

        let achievement = Achievement::get(id);
        Ok(UnlockedAchievement {
            achievement,
            unlocked_at: now,
        })
    }

    /// Get count of unlocked achievements
    pub fn unlocked_count(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("lock");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM achievements", [], |r| r.get(0))?;
        Ok(count as usize)
    }

    // ========================================
    // XP & LEVEL OPERATIONS
    // ========================================

    /// Get current player stats
    pub fn get_player_stats(&self) -> Result<PlayerStats> {
        let conn = self.conn.lock().expect("lock");
        let xp: u32 = conn.query_row(
            "SELECT total_xp FROM player_stats WHERE id = 1",
            [],
            |r| r.get(0),
        )?;
        Ok(PlayerStats::new(xp))
    }

    /// Award XP and check for level up
    pub fn award_xp(&self, amount: u32) -> Result<Option<LevelUp>> {
        let old_stats = self.get_player_stats()?;
        let new_xp = old_stats.total_xp + amount;
        let new_level = level_for_xp(new_xp);
        let new_title = title_for_level(new_level);

        let conn = self.conn.lock().expect("lock");
        conn.execute(
            "UPDATE player_stats SET total_xp = ?1, level = ?2, title = ?3 WHERE id = 1",
            (new_xp, new_level, &new_title),
        )?;
        drop(conn);

        if new_level > old_stats.level {
            Ok(Some(LevelUp {
                old_level: old_stats.level,
                new_level,
                old_title: old_stats.title.clone(),
                new_title,
            }))
        } else {
            Ok(None)
        }
    }

    // ========================================
    // STREAK OPERATIONS
    // ========================================

    /// Get current streak data
    pub fn get_streaks(&self) -> Result<Streaks> {
        let conn = self.conn.lock().expect("lock");
        let mut streaks = Streaks::default();

        // Load daily streak
        if let Ok(row) = conn.query_row(
            "SELECT current_count, best_count, last_activity_day FROM streaks WHERE streak_type = 'daily'",
            [],
            |r| Ok((r.get::<_, u32>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<String>>(2)?)),
        ) {
            streaks.daily = StreakInfo {
                current: row.0,
                best: row.1,
                last_activity_day: row.2,
            };
        }

        // Load success streak
        if let Ok(row) = conn.query_row(
            "SELECT current_count, best_count, last_activity_day FROM streaks WHERE streak_type = 'success'",
            [],
            |r| Ok((r.get::<_, u32>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<String>>(2)?)),
        ) {
            streaks.success = StreakInfo {
                current: row.0,
                best: row.1,
                last_activity_day: row.2,
            };
        }

        Ok(streaks)
    }

    /// Update daily streak (call when a job is completed)
    fn update_daily_streak(&self) -> Result<Option<u32>> {
        let streaks = self.get_streaks()?;
        let today = today_string();
        let now = Self::now_ms();

        // Check if we already counted today
        if streaks.daily.last_activity_day.as_ref() == Some(&today) {
            return Ok(None); // Already counted
        }

        // Check if streak continues or resets
        let new_count = if streaks.daily.can_extend() {
            streaks.daily.current + 1
        } else {
            1 // Reset
        };
        let new_best = new_count.max(streaks.daily.best);

        let conn = self.conn.lock().expect("lock");
        conn.execute(
            r#"
            INSERT INTO streaks (streak_type, current_count, best_count, last_activity_day, updated_at)
            VALUES ('daily', ?1, ?2, ?3, ?4)
            ON CONFLICT(streak_type) DO UPDATE SET
                current_count = ?1, best_count = ?2, last_activity_day = ?3, updated_at = ?4
            "#,
            (new_count, new_best, &today, now),
        )?;

        Ok(Some(new_count))
    }

    /// Update success streak (call after job completion)
    fn update_success_streak(&self, job_succeeded: bool) -> Result<u32> {
        let streaks = self.get_streaks()?;
        let now = Self::now_ms();

        let new_count = if job_succeeded {
            streaks.success.current + 1
        } else {
            0 // Reset on failure
        };
        let new_best = new_count.max(streaks.success.best);

        let conn = self.conn.lock().expect("lock");
        conn.execute(
            r#"
            INSERT INTO streaks (streak_type, current_count, best_count, updated_at)
            VALUES ('success', ?1, ?2, ?3)
            ON CONFLICT(streak_type) DO UPDATE SET
                current_count = ?1, best_count = ?2, updated_at = ?3
            "#,
            (new_count, new_best, now),
        )?;

        Ok(new_count)
    }

    // ========================================
    // MAIN CHECK FUNCTION
    // ========================================

    /// Check for achievements and updates after a job completes
    /// Returns all gamification events that occurred
    pub fn check_after_job(&self, job: &JobStatsRecord) -> Result<Vec<GamificationEvent>> {
        let mut events = Vec::new();
        let job_succeeded = job.status == "done" || job.status == "merged";

        // Get current state
        let unlocked = self.get_unlocked_ids()?;

        // Update streaks
        if let Some(daily_count) = self.update_daily_streak()? {
            events.push(GamificationEvent::StreakExtended {
                streak_type: StreakType::Daily,
                count: daily_count,
            });
        }
        let success_streak = self.update_success_streak(job_succeeded)?;

        // Query all stats for achievement checks
        let stats = self.query_all_stats()?;

        // Check all achievement categories
        let mut newly_unlocked = Vec::new();

        // Basic progression achievements
        newly_unlocked.extend(check_milestone_achievements(stats.total_jobs, &unlocked));
        newly_unlocked.extend(check_chain_achievements(stats.total_chains, &unlocked));
        newly_unlocked.extend(check_mode_achievements(stats.unique_modes, &unlocked));
        newly_unlocked.extend(check_agent_achievements(stats.unique_agents, &unlocked));

        // Skill achievements (job-specific)
        newly_unlocked.extend(check_skill_achievements(job, success_streak, &unlocked));

        // Time-based achievements
        newly_unlocked.extend(check_time_achievements(&unlocked));

        // Get updated daily streak for streak achievements
        let streaks = self.get_streaks()?;
        newly_unlocked.extend(check_streak_achievements(streaks.daily.current, &unlocked));
        newly_unlocked.extend(check_success_streak_achievements(success_streak, &unlocked));

        // Resource accumulation achievements
        newly_unlocked.extend(check_token_achievements(stats.total_tokens, &unlocked));
        newly_unlocked.extend(check_file_achievements(
            stats.total_file_accesses,
            stats.unique_files,
            &unlocked,
        ));
        newly_unlocked.extend(check_tool_achievements(
            stats.total_tool_calls,
            stats.unique_tools,
            &unlocked,
        ));
        newly_unlocked.extend(check_cost_achievements(stats.total_cost_usd, &unlocked));
        newly_unlocked.extend(check_lines_achievements(stats.total_lines_changed, &unlocked));
        newly_unlocked.extend(check_duration_achievements(stats.total_duration_hours, &unlocked));

        // Special achievements
        let special_ctx = self.build_special_context(&stats, job)?;
        newly_unlocked.extend(check_special_achievements(&special_ctx, &unlocked));

        // Loyalty achievements (Claude vs Codex preference)
        let loyalty_ctx = LoyaltyContext {
            claude_jobs: stats.claude_jobs,
            codex_jobs: stats.codex_jobs,
        };
        newly_unlocked.extend(check_loyalty_achievements(&loyalty_ctx, &unlocked));

        // Unlock achievements and collect XP
        let mut total_achievement_xp = 0u32;
        for id in newly_unlocked {
            let unlocked = self.unlock_achievement(id)?;
            total_achievement_xp += unlocked.achievement.xp_reward;
            events.push(GamificationEvent::AchievementUnlocked(unlocked));
        }

        // Check progressive challenges
        let mut total_challenge_xp = 0u32;
        let completed_challenges = self.check_challenges(job, &stats, &streaks)?;
        for completed in completed_challenges {
            total_challenge_xp += completed.challenge.xp_reward;
            events.push(GamificationEvent::ChallengeCompleted(completed));
        }

        // Award XP
        let base_xp = if job_succeeded {
            XpRewards::JOB_DONE
        } else {
            XpRewards::JOB_FAILED
        };

        // Streak bonus XP
        let streak_xp = if streaks.daily.current > 0 {
            XpRewards::streak_bonus(streaks.daily.current)
        } else {
            0
        };

        let total_xp = base_xp + streak_xp + total_achievement_xp + total_challenge_xp;

        if total_xp > 0 {
            let mut reason_parts = vec![format!(
                "Job {}",
                if job_succeeded { "done" } else { "failed" }
            )];
            if streak_xp > 0 {
                reason_parts.push(format!("+{} streak", streak_xp));
            }
            if total_achievement_xp > 0 {
                reason_parts.push(format!("+{} achievements", total_achievement_xp));
            }
            if total_challenge_xp > 0 {
                reason_parts.push(format!("+{} challenge", total_challenge_xp));
            }

            events.push(GamificationEvent::XpAwarded {
                amount: total_xp,
                reason: reason_parts.join(" "),
            });

            if let Some(level_up) = self.award_xp(total_xp)? {
                events.push(GamificationEvent::LevelUp(level_up));
            }
        }

        Ok(events)
    }

    /// Query all aggregate stats for achievement checks
    fn query_all_stats(&self) -> Result<AchievementStats> {
        let conn = self.conn.lock().expect("lock");
        let mut stats = AchievementStats::default();

        // Total successful jobs
        stats.total_jobs = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE status IN ('done', 'merged')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total chains
        stats.total_chains = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE mode LIKE '%chain%' OR mode LIKE '%+%'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Unique modes
        stats.unique_modes = conn
            .query_row("SELECT COUNT(DISTINCT mode) FROM job_stats", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);

        // Unique agents
        stats.unique_agents = conn
            .query_row(
                "SELECT COUNT(DISTINCT agent_type) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total tokens (input + output)
        stats.total_tokens = conn
            .query_row(
                "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total file accesses
        stats.total_file_accesses = conn
            .query_row("SELECT COUNT(*) FROM file_stats", [], |r| r.get(0))
            .unwrap_or(0);

        // Unique files accessed
        stats.unique_files = conn
            .query_row(
                "SELECT COUNT(DISTINCT file_path) FROM file_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total tool calls
        stats.total_tool_calls = conn
            .query_row("SELECT COUNT(*) FROM tool_stats", [], |r| r.get(0))
            .unwrap_or(0);

        // Unique tools used
        stats.unique_tools = conn
            .query_row(
                "SELECT COUNT(DISTINCT tool_name) FROM tool_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total cost
        stats.total_cost_usd = conn
            .query_row(
                "SELECT COALESCE(SUM(cost_usd), 0.0) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        // Total lines changed (added + removed)
        stats.total_lines_changed = conn
            .query_row(
                "SELECT COALESCE(SUM(lines_added + lines_removed), 0) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total duration in hours
        let total_duration_ms: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration_ms), 0) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        stats.total_duration_hours = total_duration_ms as f64 / 3_600_000.0;

        // Jobs today
        let today = today_string();
        stats.jobs_today = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE date(created_at/1000, 'unixepoch', 'localtime') = ?1",
                [&today],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Unique days with activity
        stats.unique_days = conn
            .query_row(
                "SELECT COUNT(DISTINCT date(created_at/1000, 'unixepoch', 'localtime')) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Agents used today
        let agents_today: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT DISTINCT agent_type FROM job_stats WHERE date(created_at/1000, 'unixepoch', 'localtime') = ?1")
                .ok();
            if let Some(ref mut s) = stmt {
                s.query_map([&today], |r| r.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        };
        stats.used_claude_today = agents_today.iter().any(|a| a.contains("claude"));
        stats.used_codex_today = agents_today.iter().any(|a| a.contains("codex"));

        // First of day count (count days where user was first job)
        // This is simplified - in a real implementation you'd track this per-day
        stats.first_of_day_count = stats.unique_days;

        // Count Claude jobs (agent_type contains "claude")
        stats.claude_jobs = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE agent_type LIKE '%claude%' AND status IN ('done', 'merged')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Count Codex jobs (agent_type contains "codex")
        stats.codex_jobs = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE agent_type LIKE '%codex%' AND status IN ('done', 'merged')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok(stats)
    }

    /// Build context for special achievement checks
    fn build_special_context(
        &self,
        stats: &AchievementStats,
        _job: &JobStatsRecord,
    ) -> Result<SpecialAchievementContext> {
        Ok(SpecialAchievementContext {
            first_of_day_count: stats.first_of_day_count,
            jobs_today: stats.jobs_today,
            used_claude_today: stats.used_claude_today,
            used_codex_today: stats.used_codex_today,
            lucky_seven_progress: 0, // TODO: Track this properly
            unique_days: stats.unique_days,
            total_successful_jobs: stats.total_jobs,
        })
    }

    // ========================================
    // CHALLENGE OPERATIONS
    // ========================================

    /// Get current challenge state
    pub fn get_challenge_state(&self) -> Result<ChallengeState> {
        let conn = self.conn.lock().expect("lock");
        let row = conn.query_row(
            "SELECT current_challenge, highest_completed FROM challenge_state WHERE id = 1",
            [],
            |r| Ok((r.get::<_, u32>(0)?, r.get::<_, u32>(1)?)),
        )?;
        Ok(ChallengeState {
            current_challenge: row.0,
            highest_completed: row.1,
        })
    }

    /// Get progress for the current active challenge
    pub fn get_current_challenge_progress(&self) -> Result<Option<ChallengeProgress>> {
        let state = self.get_challenge_state()?;

        // Check if all challenges completed
        if state.current_challenge > 50 {
            return Ok(None);
        }

        let challenge = match Challenge::get_by_number(state.current_challenge) {
            Some(c) => c,
            None => return Ok(None),
        };

        let stats = self.query_all_stats()?;
        let streaks = self.get_streaks()?;

        let (current, target) = self.get_challenge_values(challenge, &stats, &streaks);

        let conn = self.conn.lock().expect("lock");
        let completed_at: Option<i64> = conn
            .query_row(
                "SELECT completed_at FROM progressive_challenges WHERE id = ?1",
                [challenge.id.as_str()],
                |r| r.get(0),
            )
            .ok()
            .flatten();

        Ok(Some(ChallengeProgress {
            challenge,
            current_value: current,
            target_value: target,
            completed: completed_at.is_some(),
            completed_at,
        }))
    }

    /// Get all challenge progress
    pub fn get_all_challenges_progress(&self) -> Result<Vec<ChallengeProgress>> {
        let stats = self.query_all_stats()?;
        let streaks = self.get_streaks()?;
        let state = self.get_challenge_state()?;

        let conn = self.conn.lock().expect("lock");
        let mut results = Vec::new();

        for challenge in super::challenges::CHALLENGES.iter() {
            let (current, target) = self.get_challenge_values(challenge, &stats, &streaks);

            let completed_at: Option<i64> = conn
                .query_row(
                    "SELECT completed_at FROM progressive_challenges WHERE id = ?1",
                    [challenge.id.as_str()],
                    |r| r.get(0),
                )
                .ok()
                .flatten();

            // A challenge is only "available" if its number <= current_challenge
            let is_available = challenge.id.number() <= state.current_challenge;
            let is_completed = completed_at.is_some();

            results.push(ChallengeProgress {
                challenge,
                current_value: if is_available { current } else { 0 },
                target_value: target,
                completed: is_completed,
                completed_at,
            });
        }

        Ok(results)
    }

    /// Check and complete challenges after a job
    fn check_challenges(
        &self,
        job: &JobStatsRecord,
        stats: &AchievementStats,
        streaks: &Streaks,
    ) -> Result<Vec<CompletedChallenge>> {
        let mut completed = Vec::new();
        let state = self.get_challenge_state()?;

        // Check if all challenges already completed
        if state.current_challenge > 50 {
            return Ok(completed);
        }

        let current = match Challenge::get_by_number(state.current_challenge) {
            Some(c) => c,
            None => return Ok(completed),
        };

        // Check if the current challenge is now complete
        if self.is_challenge_complete(current, job, stats, streaks) {
            let completed_challenge = self.complete_challenge(current)?;
            completed.push(completed_challenge);
        }

        Ok(completed)
    }

    /// Check if a specific challenge's requirements are met
    fn is_challenge_complete(
        &self,
        challenge: &Challenge,
        job: &JobStatsRecord,
        stats: &AchievementStats,
        streaks: &Streaks,
    ) -> bool {
        match challenge.requirement {
            ChallengeRequirement::TotalJobs(n) => stats.total_jobs >= n as u64,
            ChallengeRequirement::SuccessStreak(n) => streaks.success.current >= n,
            ChallengeRequirement::DailyStreak(n) => streaks.daily.current >= n,
            ChallengeRequirement::UniqueModes(n) => stats.unique_modes >= n as u64,
            ChallengeRequirement::UniqueTools(n) => stats.unique_tools >= n as u64,
            ChallengeRequirement::UniqueAgents(n) => stats.unique_agents >= n as u64,
            ChallengeRequirement::JobUnderMs(ms) => {
                job.status == "done" && (job.duration_ms as u64) < ms
            }
            ChallengeRequirement::FilesAccessed(n) => stats.total_file_accesses >= n as u64,
            ChallengeRequirement::TotalTokens(n) => stats.total_tokens >= n,
            ChallengeRequirement::TotalChains(n) => stats.total_chains >= n as u64,
            ChallengeRequirement::WeekendJob => is_weekend() && job.status == "done",
        }
    }

    /// Complete a challenge and advance to the next one
    fn complete_challenge(&self, challenge: &Challenge) -> Result<CompletedChallenge> {
        let now = Self::now_ms();
        let challenge_number = challenge.id.number();
        let challenge_id_str = challenge.id.as_str();

        let conn = self.conn.lock().expect("lock");

        // Mark challenge as completed
        conn.execute(
            "INSERT OR REPLACE INTO progressive_challenges (id, completed_at) VALUES (?1, ?2)",
            (challenge_id_str, now),
        )?;

        // Advance to next challenge
        let next_challenge = challenge_number + 1;
        conn.execute(
            "UPDATE challenge_state SET current_challenge = ?1, highest_completed = ?2 WHERE id = 1",
            (next_challenge, challenge_number),
        )?;

        drop(conn);

        // Get a static reference to the challenge for the return value
        let static_challenge = Challenge::get(challenge.id);

        Ok(CompletedChallenge {
            challenge: static_challenge,
            completed_at: now,
        })
    }

    /// Get current and target values for a challenge
    fn get_challenge_values(
        &self,
        challenge: &Challenge,
        stats: &AchievementStats,
        streaks: &Streaks,
    ) -> (u64, u64) {
        match challenge.requirement {
            ChallengeRequirement::TotalJobs(n) => (stats.total_jobs, n as u64),
            ChallengeRequirement::SuccessStreak(n) => (streaks.success.current as u64, n as u64),
            ChallengeRequirement::DailyStreak(n) => (streaks.daily.current as u64, n as u64),
            ChallengeRequirement::UniqueModes(n) => (stats.unique_modes, n as u64),
            ChallengeRequirement::UniqueTools(n) => (stats.unique_tools, n as u64),
            ChallengeRequirement::UniqueAgents(n) => (stats.unique_agents, n as u64),
            ChallengeRequirement::JobUnderMs(_) => (1, 1), // Binary: done or not
            ChallengeRequirement::FilesAccessed(n) => (stats.total_file_accesses, n as u64),
            ChallengeRequirement::TotalTokens(n) => (stats.total_tokens, n),
            ChallengeRequirement::TotalChains(n) => (stats.total_chains, n as u64),
            ChallengeRequirement::WeekendJob => (if is_weekend() { 1 } else { 0 }, 1),
        }
    }
}

/// Current challenge state
#[derive(Debug, Clone, Default)]
pub struct ChallengeState {
    pub current_challenge: u32,
    pub highest_completed: u32,
}
