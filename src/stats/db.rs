//! SQLite database connection and schema management for statistics
//!
//! Manages the `~/.kyco/stats.db` database with automatic schema migration.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::config::Config;

/// Database wrapper with connection pooling
#[derive(Clone)]
pub struct StatsDb {
    /// Database connection - pub(crate) for AchievementManager access
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl StatsDb {
    /// Open or create the stats database at the default location (~/.kyco/stats.db)
    pub fn open_default() -> Result<Self> {
        let db_path = Config::global_config_dir().join("stats.db");
        Self::open(&db_path)
    }

    /// Open or create the stats database at a specific path
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create stats dir: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open stats db: {}", path.display()))?;

        // Enable WAL mode for concurrent access from Rust and Bridge
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Get a reference to the connection (for queries)
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("Stats DB lock poisoned")
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(SCHEMA_SQL)?;
        drop(conn);
        self.run_migrations()?;
        Ok(())
    }

    /// Run any pending migrations
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn();

        // Get current schema version
        let version: i32 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
            .unwrap_or(0);

        // Migration 2: Rename workspace_id to workspace_path
        if version < 2 {
            // Check if workspace_id column exists (old schema)
            let has_workspace_id: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('job_stats') WHERE name = 'workspace_id'")
                .and_then(|mut s| s.query_row([], |r| r.get::<_, i32>(0)))
                .map(|c| c > 0)
                .unwrap_or(false);

            let has_workspace_path: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('job_stats') WHERE name = 'workspace_path'")
                .and_then(|mut s| s.query_row([], |r| r.get::<_, i32>(0)))
                .map(|c| c > 0)
                .unwrap_or(false);

            if has_workspace_id && !has_workspace_path {
                // Rename workspace_id to workspace_path
                conn.execute_batch(r#"
                    ALTER TABLE job_stats RENAME COLUMN workspace_id TO workspace_path;
                    CREATE INDEX IF NOT EXISTS idx_job_workspace ON job_stats(workspace_path);
                "#)?;
            } else if !has_workspace_path {
                // Add workspace_path if neither exists
                conn.execute_batch(r#"
                    ALTER TABLE job_stats ADD COLUMN workspace_path TEXT;
                    CREATE INDEX IF NOT EXISTS idx_job_workspace ON job_stats(workspace_path);
                "#)?;
            }

            conn.execute("INSERT OR REPLACE INTO schema_version VALUES (2)", [])?;
        }

        // Migration 3: Add gamification tables
        if version < 3 {
            conn.execute_batch(
                r#"
                -- Unlocked achievements
                CREATE TABLE IF NOT EXISTS achievements (
                    id TEXT PRIMARY KEY,
                    unlocked_at INTEGER NOT NULL,
                    progress INTEGER DEFAULT 0
                );

                -- Streak tracking
                CREATE TABLE IF NOT EXISTS streaks (
                    streak_type TEXT PRIMARY KEY,
                    current_count INTEGER DEFAULT 0,
                    best_count INTEGER DEFAULT 0,
                    last_activity_day TEXT,
                    updated_at INTEGER
                );

                -- Player stats (singleton)
                CREATE TABLE IF NOT EXISTS player_stats (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    total_xp INTEGER DEFAULT 0,
                    level INTEGER DEFAULT 1,
                    title TEXT DEFAULT 'Apprentice'
                );
                INSERT OR IGNORE INTO player_stats (id) VALUES (1);

                -- Weekly challenges
                CREATE TABLE IF NOT EXISTS weekly_challenges (
                    week_start TEXT PRIMARY KEY,
                    challenge_ids TEXT,
                    current_step INTEGER DEFAULT 0,
                    completed_at INTEGER,
                    player_level INTEGER
                );

                -- Challenge progress
                CREATE TABLE IF NOT EXISTS challenge_progress (
                    id TEXT PRIMARY KEY,
                    week_start TEXT NOT NULL,
                    progress INTEGER DEFAULT 0,
                    completed_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_challenge_week ON challenge_progress(week_start);
                "#,
            )?;
            conn.execute("INSERT OR REPLACE INTO schema_version VALUES (3)", [])?;
        }

        // Migration 4: Add progressive challenges table
        if version < 4 {
            conn.execute_batch(
                r#"
                -- Progressive challenges (sequential unlock, permanent progress)
                CREATE TABLE IF NOT EXISTS progressive_challenges (
                    id TEXT PRIMARY KEY,          -- Challenge ID (e.g., "ch_first_steps")
                    completed_at INTEGER,         -- Timestamp when completed (NULL if not completed)
                    progress INTEGER DEFAULT 0    -- Current progress towards goal
                );

                -- Track which challenge is currently active
                CREATE TABLE IF NOT EXISTS challenge_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    current_challenge INTEGER DEFAULT 1,  -- Challenge number (1-50)
                    highest_completed INTEGER DEFAULT 0   -- Highest completed challenge number
                );
                INSERT OR IGNORE INTO challenge_state (id) VALUES (1);
                "#,
            )?;
            conn.execute("INSERT OR REPLACE INTO schema_version VALUES (4)", [])?;
        }

        Ok(())
    }

    /// Delete all statistics data (reset to empty state)
    /// Note: This does NOT reset achievements/XP - use reset_achievements() for that
    pub fn reset_all(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            r#"
            DELETE FROM job_stats;
            DELETE FROM tool_stats;
            DELETE FROM file_stats;
            DELETE FROM daily_stats;
            DELETE FROM mode_stats;
            DELETE FROM tool_usage_stats;
            DELETE FROM file_access_stats;
            "#,
        )?;
        Ok(())
    }

    /// Delete all gamification data (achievements, XP, streaks, challenges)
    pub fn reset_achievements(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            r#"
            DELETE FROM achievements;
            DELETE FROM streaks;
            DELETE FROM challenge_progress;
            DELETE FROM weekly_challenges;
            DELETE FROM progressive_challenges;
            UPDATE player_stats SET total_xp = 0, level = 1, title = 'Apprentice' WHERE id = 1;
            UPDATE challenge_state SET current_challenge = 1, highest_completed = 0 WHERE id = 1;
            "#,
        )?;
        Ok(())
    }
}

/// SQL schema for the stats database
const SCHEMA_SQL: &str = r#"
-- Job statistics (one row per completed job)
CREATE TABLE IF NOT EXISTS job_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL UNIQUE,
    session_id TEXT,
    mode TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    status TEXT NOT NULL,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    cost_usd REAL DEFAULT 0.0,
    duration_ms INTEGER DEFAULT 0,
    files_changed INTEGER DEFAULT 0,
    lines_added INTEGER DEFAULT 0,
    lines_removed INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER,
    day_bucket TEXT NOT NULL,
    interval_bucket TEXT NOT NULL,
    workspace_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_job_day ON job_stats(day_bucket);
CREATE INDEX IF NOT EXISTS idx_job_created_at ON job_stats(created_at);
CREATE INDEX IF NOT EXISTS idx_job_mode ON job_stats(mode);
CREATE INDEX IF NOT EXISTS idx_job_agent ON job_stats(agent_type);
CREATE INDEX IF NOT EXISTS idx_job_workspace ON job_stats(workspace_path);

-- Tool call statistics
CREATE TABLE IF NOT EXISTS tool_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL,
    session_id TEXT,
    tool_name TEXT NOT NULL,
    tool_use_id TEXT,
    success INTEGER NOT NULL DEFAULT 1,
    timestamp INTEGER NOT NULL,
    day_bucket TEXT NOT NULL,
    interval_bucket TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tool_name ON tool_stats(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_day ON tool_stats(day_bucket);
CREATE INDEX IF NOT EXISTS idx_tool_job ON tool_stats(job_id);

-- File access statistics
CREATE TABLE IF NOT EXISTS file_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL,
    session_id TEXT,
    file_path TEXT NOT NULL,
    access_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    day_bucket TEXT NOT NULL,
    interval_bucket TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_file_path ON file_stats(file_path);
CREATE INDEX IF NOT EXISTS idx_file_day ON file_stats(day_bucket);

-- Daily aggregates (for fast graph queries)
CREATE TABLE IF NOT EXISTS daily_stats (
    day_bucket TEXT PRIMARY KEY,
    total_jobs INTEGER DEFAULT 0,
    done_jobs INTEGER DEFAULT 0,
    failed_jobs INTEGER DEFAULT 0,
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    total_cache_read INTEGER DEFAULT 0,
    total_cache_write INTEGER DEFAULT 0,
    total_cost_usd REAL DEFAULT 0.0,
    claude_jobs INTEGER DEFAULT 0,
    codex_jobs INTEGER DEFAULT 0,
    total_files_changed INTEGER DEFAULT 0,
    total_tool_calls INTEGER DEFAULT 0,
    last_updated INTEGER NOT NULL
);

-- Mode usage aggregates
CREATE TABLE IF NOT EXISTS mode_stats (
    mode TEXT PRIMARY KEY,
    total_jobs INTEGER DEFAULT 0,
    done_jobs INTEGER DEFAULT 0,
    failed_jobs INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost_usd REAL DEFAULT 0.0,
    last_updated INTEGER NOT NULL
);

-- Tool usage aggregates
CREATE TABLE IF NOT EXISTS tool_usage_stats (
    tool_name TEXT PRIMARY KEY,
    total_calls INTEGER DEFAULT 0,
    successful_calls INTEGER DEFAULT 0,
    failed_calls INTEGER DEFAULT 0,
    last_updated INTEGER NOT NULL
);

-- File access aggregates
CREATE TABLE IF NOT EXISTS file_access_stats (
    file_path TEXT PRIMARY KEY,
    total_accesses INTEGER DEFAULT 0,
    read_count INTEGER DEFAULT 0,
    write_count INTEGER DEFAULT 0,
    edit_count INTEGER DEFAULT 0,
    last_updated INTEGER NOT NULL
);

-- Schema version
CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
INSERT OR IGNORE INTO schema_version VALUES (3);

-- ============================================
-- GAMIFICATION TABLES
-- ============================================

-- Unlocked achievements
CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    unlocked_at INTEGER NOT NULL,
    progress INTEGER DEFAULT 0
);

-- Streak tracking (daily usage, success streaks)
CREATE TABLE IF NOT EXISTS streaks (
    streak_type TEXT PRIMARY KEY,
    current_count INTEGER DEFAULT 0,
    best_count INTEGER DEFAULT 0,
    last_activity_day TEXT,
    updated_at INTEGER
);

-- Player stats (XP, level, title) - singleton row
CREATE TABLE IF NOT EXISTS player_stats (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    total_xp INTEGER DEFAULT 0,
    level INTEGER DEFAULT 1,
    title TEXT DEFAULT 'Apprentice'
);
INSERT OR IGNORE INTO player_stats (id) VALUES (1);

-- Weekly challenges
CREATE TABLE IF NOT EXISTS weekly_challenges (
    week_start TEXT PRIMARY KEY,
    challenge_ids TEXT,
    current_step INTEGER DEFAULT 0,
    completed_at INTEGER,
    player_level INTEGER
);

-- Challenge progress tracking
CREATE TABLE IF NOT EXISTS challenge_progress (
    id TEXT PRIMARY KEY,
    week_start TEXT NOT NULL,
    progress INTEGER DEFAULT 0,
    completed_at INTEGER,
    FOREIGN KEY (week_start) REFERENCES weekly_challenges(week_start)
);
CREATE INDEX IF NOT EXISTS idx_challenge_week ON challenge_progress(week_start);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_and_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_stats.db");
        let db = StatsDb::open(&db_path).unwrap();

        // Verify tables exist
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"job_stats".to_string()));
        assert!(tables.contains(&"tool_stats".to_string()));
        assert!(tables.contains(&"file_stats".to_string()));
    }
}
