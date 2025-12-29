//! Stats recorder - writes statistics to the database
//!
//! Handles recording of job completions, tool calls, and file accesses.

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use super::db::StatsDb;
use super::models::{FileAccessType, FileStatsRecord, JobStatsRecord, ToolStatsRecord};
use super::time_bucket::{day_bucket, interval_bucket};

/// Records statistics to the database
#[derive(Clone)]
pub struct StatsRecorder {
    db: StatsDb,
}

impl StatsRecorder {
    pub fn new(db: StatsDb) -> Self {
        Self { db }
    }

    /// Record a completed job's statistics
    pub fn record_job(&self, record: &JobStatsRecord) -> Result<()> {
        let day = day_bucket(record.created_at);
        let interval = interval_bucket(record.created_at);
        let now = Utc::now().timestamp_millis();

        let conn = self.db.conn();
        conn.execute(
            r#"INSERT OR REPLACE INTO job_stats
               (job_id, session_id, mode, agent_id, agent_type, status,
                input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                cost_usd, duration_ms, files_changed, lines_added, lines_removed,
                created_at, started_at, finished_at, day_bucket, interval_bucket, workspace_id)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"#,
            rusqlite::params![
                record.job_id, record.session_id, record.mode, record.agent_id, record.agent_type,
                record.status, record.input_tokens, record.output_tokens, record.cache_read_tokens,
                record.cache_write_tokens, record.cost_usd, record.duration_ms, record.files_changed,
                record.lines_added, record.lines_removed, record.created_at, record.started_at,
                record.finished_at, day, interval, record.workspace_id,
            ],
        )?;

        // Update aggregates using the same connection (no deadlock!)
        Self::update_daily_stats(&conn, &day, record, now)?;
        Self::update_mode_stats(&conn, &record.mode, record, now)?;
        Ok(())
    }

    /// Record a tool call
    pub fn record_tool_call(&self, record: &ToolStatsRecord) -> Result<()> {
        let day = day_bucket(record.timestamp);
        let interval = interval_bucket(record.timestamp);
        let now = Utc::now().timestamp_millis();

        let conn = self.db.conn();
        conn.execute(
            r#"INSERT INTO tool_stats
               (job_id, session_id, tool_name, tool_use_id, success, timestamp, day_bucket, interval_bucket)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            rusqlite::params![
                record.job_id, record.session_id, record.tool_name, record.tool_use_id,
                record.success as i32, record.timestamp, day, interval,
            ],
        )?;
        Self::update_tool_stats(&conn, &record.tool_name, record.success, now)?;
        Ok(())
    }

    /// Record a file access
    pub fn record_file_access(&self, record: &FileStatsRecord) -> Result<()> {
        let day = day_bucket(record.timestamp);
        let interval = interval_bucket(record.timestamp);
        let normalized = normalize_path(&record.file_path);
        let now = Utc::now().timestamp_millis();

        let conn = self.db.conn();
        conn.execute(
            r#"INSERT INTO file_stats
               (job_id, session_id, file_path, access_type, timestamp, day_bucket, interval_bucket)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            rusqlite::params![
                record.job_id, record.session_id, normalized, record.access_type.as_str(),
                record.timestamp, day, interval,
            ],
        )?;
        Self::update_file_stats(&conn, &normalized, record.access_type, now)?;
        Ok(())
    }

    fn update_daily_stats(conn: &Connection, day: &str, job: &JobStatsRecord, now: i64) -> Result<()> {
        let is_done = job.status == "done" || job.status == "merged";
        let is_failed = job.status == "failed";
        let is_claude = job.agent_type == "claude";
        conn.execute(
            r#"INSERT INTO daily_stats (day_bucket, total_jobs, done_jobs, failed_jobs,
                   total_input_tokens, total_output_tokens, total_cost_usd,
                   claude_jobs, codex_jobs, total_files_changed, last_updated)
               VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               ON CONFLICT(day_bucket) DO UPDATE SET
                   total_jobs = total_jobs + 1, done_jobs = done_jobs + ?2, failed_jobs = failed_jobs + ?3,
                   total_input_tokens = total_input_tokens + ?4, total_output_tokens = total_output_tokens + ?5,
                   total_cost_usd = total_cost_usd + ?6, claude_jobs = claude_jobs + ?7,
                   codex_jobs = codex_jobs + ?8, total_files_changed = total_files_changed + ?9, last_updated = ?10"#,
            rusqlite::params![
                day, is_done as i32, is_failed as i32, job.input_tokens, job.output_tokens,
                job.cost_usd, is_claude as i32, (!is_claude) as i32, job.files_changed, now,
            ],
        )?;
        Ok(())
    }

    fn update_mode_stats(conn: &Connection, mode: &str, job: &JobStatsRecord, now: i64) -> Result<()> {
        let is_done = job.status == "done" || job.status == "merged";
        let is_failed = job.status == "failed";
        let tokens = job.input_tokens + job.output_tokens;
        conn.execute(
            r#"INSERT INTO mode_stats (mode, total_jobs, done_jobs, failed_jobs, total_tokens, total_cost_usd, last_updated)
               VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(mode) DO UPDATE SET
                   total_jobs = total_jobs + 1, done_jobs = done_jobs + ?2, failed_jobs = failed_jobs + ?3,
                   total_tokens = total_tokens + ?4, total_cost_usd = total_cost_usd + ?5, last_updated = ?6"#,
            rusqlite::params![mode, is_done as i32, is_failed as i32, tokens, job.cost_usd, now],
        )?;
        Ok(())
    }

    fn update_tool_stats(conn: &Connection, tool: &str, success: bool, now: i64) -> Result<()> {
        conn.execute(
            r#"INSERT INTO tool_usage_stats (tool_name, total_calls, successful_calls, failed_calls, last_updated)
               VALUES (?1, 1, ?2, ?3, ?4)
               ON CONFLICT(tool_name) DO UPDATE SET
                   total_calls = total_calls + 1, successful_calls = successful_calls + ?2,
                   failed_calls = failed_calls + ?3, last_updated = ?4"#,
            rusqlite::params![tool, success as i32, (!success) as i32, now],
        )?;
        Ok(())
    }

    fn update_file_stats(conn: &Connection, path: &str, access: FileAccessType, now: i64) -> Result<()> {
        let (r, w, e) = match access {
            FileAccessType::Read => (1, 0, 0),
            FileAccessType::Write => (0, 1, 0),
            FileAccessType::Edit => (0, 0, 1),
        };
        conn.execute(
            r#"INSERT INTO file_access_stats (file_path, total_accesses, read_count, write_count, edit_count, last_updated)
               VALUES (?1, 1, ?2, ?3, ?4, ?5)
               ON CONFLICT(file_path) DO UPDATE SET
                   total_accesses = total_accesses + 1, read_count = read_count + ?2,
                   write_count = write_count + ?3, edit_count = edit_count + ?4, last_updated = ?5"#,
            rusqlite::params![path, r, w, e, now],
        )?;
        Ok(())
    }
}

/// Normalize file path for consistent storage
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}
