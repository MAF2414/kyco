//! Summary queries for statistics

use anyhow::Result;
use chrono::Utc;

use super::StatsQuery;
use crate::stats::models::{DailyStatsView, StatsSummary, TimeRange};
use crate::stats::time_bucket::day_bucket;

impl StatsQuery {
    /// Get a complete summary for the dashboard
    pub fn get_summary(&self, time_range: TimeRange) -> Result<StatsSummary> {
        let conn = self.db.conn();
        let cutoff = self.cutoff_day(time_range);

        let (total_jobs, total_tokens, total_cost) = self.get_totals(&conn, cutoff.as_deref())?;
        let jobs_by_status = self.get_jobs_by_status(&conn, cutoff.as_deref())?;
        let jobs_by_agent = self.get_jobs_by_agent(&conn, cutoff.as_deref())?;
        let top_modes = self.get_top_modes(&conn, 10)?;
        let top_tools = self.get_top_tools(&conn, 10)?;
        let top_files = self.get_top_files(&conn, 10)?;
        let daily_stats = self.get_daily_stats(&conn, time_range)?;

        let total_tool_calls: u64 = conn
            .query_row("SELECT COALESCE(SUM(total_calls), 0) FROM tool_usage_stats", [], |r| {
                r.get(0)
            })?;

        Ok(StatsSummary {
            total_jobs,
            total_tokens,
            total_cost_usd: total_cost,
            total_tool_calls,
            jobs_by_status,
            jobs_by_agent,
            top_modes,
            top_tools,
            top_files,
            daily_stats,
        })
    }

    fn cutoff_day(&self, range: TimeRange) -> Option<String> {
        range.days().map(|days| {
            let cutoff_ms = Utc::now().timestamp_millis() - (days as i64 * 24 * 60 * 60 * 1000);
            day_bucket(cutoff_ms)
        })
    }

    fn get_totals(
        &self,
        conn: &rusqlite::Connection,
        cutoff: Option<&str>,
    ) -> Result<(u64, u64, f64)> {
        let result = if let Some(c) = cutoff {
            conn.query_row(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens + output_tokens), 0), COALESCE(SUM(cost_usd), 0) FROM job_stats WHERE day_bucket >= ?",
                [c],
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?, row.get::<_, f64>(2)?)),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens + output_tokens), 0), COALESCE(SUM(cost_usd), 0) FROM job_stats",
                [],
                |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?, row.get::<_, f64>(2)?)),
            )?
        };
        Ok(result)
    }

    fn get_jobs_by_status(
        &self,
        conn: &rusqlite::Connection,
        cutoff: Option<&str>,
    ) -> Result<Vec<(String, u64)>> {
        if let Some(c) = cutoff {
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM job_stats WHERE day_bucket >= ? GROUP BY status ORDER BY COUNT(*) DESC",
            )?;
            let rows = stmt.query_map([c], |row| Ok((row.get(0)?, row.get(1)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn.prepare(
                "SELECT status, COUNT(*) FROM job_stats GROUP BY status ORDER BY COUNT(*) DESC",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }

    fn get_jobs_by_agent(
        &self,
        conn: &rusqlite::Connection,
        cutoff: Option<&str>,
    ) -> Result<Vec<(String, u64, u64)>> {
        if let Some(c) = cutoff {
            let mut stmt = conn.prepare(
                "SELECT agent_type, COUNT(*), COALESCE(SUM(input_tokens + output_tokens), 0) FROM job_stats WHERE day_bucket >= ? GROUP BY agent_type",
            )?;
            let rows = stmt.query_map([c], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn.prepare(
                "SELECT agent_type, COUNT(*), COALESCE(SUM(input_tokens + output_tokens), 0) FROM job_stats GROUP BY agent_type",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }

    fn get_top_modes(&self, conn: &rusqlite::Connection, limit: usize) -> Result<Vec<(String, u64)>> {
        let mut stmt = conn.prepare(
            "SELECT mode, total_jobs FROM mode_stats ORDER BY total_jobs DESC LIMIT ?",
        )?;
        let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn get_top_tools(&self, conn: &rusqlite::Connection, limit: usize) -> Result<Vec<(String, u64)>> {
        // First try aggregate table
        let mut stmt = conn.prepare(
            "SELECT tool_name, total_calls FROM tool_usage_stats ORDER BY total_calls DESC LIMIT ?",
        )?;
        let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let results: Vec<(String, u64)> = rows.filter_map(|r| r.ok()).collect();

        // If aggregate is empty, query raw table directly
        if results.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT tool_name, COUNT(*) as cnt FROM tool_stats GROUP BY tool_name ORDER BY cnt DESC LIMIT ?",
            )?;
            let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            Ok(results)
        }
    }

    fn get_top_files(&self, conn: &rusqlite::Connection, limit: usize) -> Result<Vec<(String, u64)>> {
        // First try aggregate table
        let mut stmt = conn.prepare(
            "SELECT file_path, total_accesses FROM file_access_stats ORDER BY total_accesses DESC LIMIT ?",
        )?;
        let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let results: Vec<(String, u64)> = rows.filter_map(|r| r.ok()).collect();

        // If aggregate is empty, query raw table directly
        if results.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT file_path, COUNT(*) as cnt FROM file_stats GROUP BY file_path ORDER BY cnt DESC LIMIT ?",
            )?;
            let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            Ok(results)
        }
    }

    fn get_daily_stats(
        &self,
        conn: &rusqlite::Connection,
        range: TimeRange,
    ) -> Result<Vec<DailyStatsView>> {
        let days = range.days().unwrap_or(365);
        let cutoff_ms = Utc::now().timestamp_millis() - (days as i64 * 24 * 60 * 60 * 1000);
        let cutoff = day_bucket(cutoff_ms);

        let mut stmt = conn.prepare(
            "SELECT day_bucket, total_jobs, done_jobs, failed_jobs,
                    total_input_tokens, total_output_tokens, total_cost_usd,
                    claude_jobs, codex_jobs, total_tool_calls
             FROM daily_stats WHERE day_bucket >= ? ORDER BY day_bucket ASC",
        )?;

        let rows = stmt.query_map([cutoff], |row| {
            Ok(DailyStatsView {
                day: row.get(0)?,
                total_jobs: row.get(1)?,
                done_jobs: row.get(2)?,
                failed_jobs: row.get(3)?,
                total_input_tokens: row.get(4)?,
                total_output_tokens: row.get(5)?,
                total_cost_usd: row.get(6)?,
                claude_jobs: row.get(7)?,
                codex_jobs: row.get(8)?,
                total_tool_calls: row.get(9)?,
            })
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
