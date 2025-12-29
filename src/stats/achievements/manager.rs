//! Achievement Manager - Core gamification logic
//!
//! Handles achievement checking, XP awards, streak updates, and database operations.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::Connection;

use super::checker::{
    check_agent_achievements, check_chain_achievements, check_milestone_achievements,
    check_mode_achievements, check_skill_achievements, check_streak_achievements,
    check_time_achievements,
};
use super::definitions::{Achievement, AchievementId};
use super::levels::{Level, PlayerStats, XpRewards};
use super::streaks::{today_string, StreakInfo, StreakType, Streaks};
use crate::stats::models::JobStatsRecord;

/// An achievement that was just unlocked
#[derive(Debug, Clone)]
pub struct UnlockedAchievement {
    pub achievement: &'static Achievement,
    pub unlocked_at: i64,
}

/// A level up event
#[derive(Debug, Clone)]
pub struct LevelUp {
    pub old_level: u32,
    pub new_level: u32,
    pub new_title: String,
}

/// Events that can happen during gamification checks
#[derive(Debug, Clone)]
pub enum GamificationEvent {
    AchievementUnlocked(UnlockedAchievement),
    LevelUp(LevelUp),
    StreakExtended { streak_type: StreakType, count: u32 },
    XpAwarded { amount: u32, reason: String },
}

/// Main manager for all gamification features
pub struct AchievementManager {
    conn: Arc<Mutex<Connection>>,
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
        let new_level_info = Level::for_xp(new_xp);

        let conn = self.conn.lock().expect("lock");
        conn.execute(
            "UPDATE player_stats SET total_xp = ?1, level = ?2, title = ?3 WHERE id = 1",
            (new_xp, new_level_info.level, new_level_info.title),
        )?;
        drop(conn);

        if new_level_info.level > old_stats.level {
            Ok(Some(LevelUp {
                old_level: old_stats.level,
                new_level: new_level_info.level,
                new_title: new_level_info.title.to_string(),
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

        // Query stats for achievement checks
        let (total_jobs, total_chains, unique_modes, unique_agents) = self.query_stats()?;

        // Check all achievement categories
        let mut newly_unlocked = Vec::new();

        newly_unlocked.extend(check_milestone_achievements(total_jobs, &unlocked));
        newly_unlocked.extend(check_chain_achievements(total_chains, &unlocked));
        newly_unlocked.extend(check_mode_achievements(unique_modes, &unlocked));
        newly_unlocked.extend(check_agent_achievements(unique_agents, &unlocked));
        newly_unlocked.extend(check_skill_achievements(job, success_streak, &unlocked));
        newly_unlocked.extend(check_time_achievements(&unlocked));

        // Get updated daily streak for streak achievements
        let streaks = self.get_streaks()?;
        newly_unlocked.extend(check_streak_achievements(streaks.daily.current, &unlocked));

        // Unlock achievements and collect XP
        let mut total_achievement_xp = 0u32;
        for id in newly_unlocked {
            let unlocked = self.unlock_achievement(id)?;
            total_achievement_xp += unlocked.achievement.xp_reward;
            events.push(GamificationEvent::AchievementUnlocked(unlocked));
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

        let total_xp = base_xp + streak_xp + total_achievement_xp;

        if total_xp > 0 {
            events.push(GamificationEvent::XpAwarded {
                amount: total_xp,
                reason: format!(
                    "Job {} (+{} streak, +{} achievements)",
                    if job_succeeded { "done" } else { "failed" },
                    streak_xp,
                    total_achievement_xp
                ),
            });

            if let Some(level_up) = self.award_xp(total_xp)? {
                events.push(GamificationEvent::LevelUp(level_up));
            }
        }

        Ok(events)
    }

    /// Query aggregate stats for achievement checks
    fn query_stats(&self) -> Result<(u64, u64, u64, u64)> {
        let conn = self.conn.lock().expect("lock");

        // Total successful jobs
        let total_jobs: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE status IN ('done', 'merged')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Total chains (jobs with mode containing "chain" or from chain table)
        // For now, approximate by counting jobs from known chain patterns
        let total_chains: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_stats WHERE mode LIKE '%chain%' OR mode LIKE '%+%'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Unique modes used
        let unique_modes: u64 = conn
            .query_row("SELECT COUNT(DISTINCT mode) FROM job_stats", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);

        // Unique agents used
        let unique_agents: u64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT agent_type) FROM job_stats",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok((total_jobs, total_chains, unique_modes, unique_agents))
    }
}
