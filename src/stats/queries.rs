//! Stats query functions for reading aggregated data
//!
//! Provides efficient queries for the Stats GUI view.

use anyhow::Result;
use chrono::Utc;

use super::db::StatsDb;
use super::models::{
    AgentStats, DailyStatsView, DashboardFilter, DashboardSummary, ModeChainStats, StatsSummary,
    TimeRange, TokenBreakdown, TrendValue,
};
use super::time_bucket::day_bucket;

/// Query interface for statistics
pub struct StatsQuery {
    db: StatsDb,
}

/// Internal struct for period statistics query result
struct PeriodStats {
    succeeded_jobs: f64,
    total_tokens: f64,
    total_cost: f64,
    total_bytes: f64,
    avg_duration: f64,
    wall_clock: f64,
    input_tokens: f64,
    output_tokens: f64,
    cached_tokens: f64,
    failed_jobs: f64,
}

impl StatsQuery {
    pub fn new(db: StatsDb) -> Self {
        Self { db }
    }

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

    // ========================================================================
    // Dashboard V2 Queries
    // ========================================================================

    /// Get complete dashboard summary with optional filters
    pub fn get_dashboard(&self, range: TimeRange, filter: &DashboardFilter) -> Result<DashboardSummary> {
        let conn = self.db.conn();
        let (cutoff, prev_cutoff) = self.cutoff_days_with_prev(range);

        // Build WHERE clause based on filter
        let where_clause = self.build_where_clause(filter, cutoff.as_deref());
        let prev_where = self.build_where_clause(filter, prev_cutoff.as_deref());

        // Get current period stats
        let current = self.query_period_stats(&conn, &where_clause)?;
        let previous = self.query_period_stats(&conn, &prev_where)?;

        // Get tool calls and file accesses
        let current_tools = self.query_total_tool_calls(&conn, &where_clause).unwrap_or(0);
        let previous_tools = self.query_total_tool_calls(&conn, &prev_where).unwrap_or(0);
        let current_files = self.query_total_file_accesses(&conn, &where_clause).unwrap_or(0);
        let previous_files = self.query_total_file_accesses(&conn, &prev_where).unwrap_or(0);

        // Get token breakdown
        let tokens = self.query_token_breakdown(&conn, &where_clause)?;

        // Get agent breakdown
        let agents = self.query_agent_stats(&conn, &where_clause)?;

        // Get mode stats
        let modes = self.query_mode_stats(&conn, &where_clause)?;

        // Get top tools/files (unfiltered for now)
        let top_tools = self.get_top_tools(&conn, 8)?;
        let top_files = self.get_top_files(&conn, 8)?;

        // Get available filter options
        let available_agents = self.get_available_agents(&conn)?;
        let available_modes = self.get_available_modes(&conn)?;
        let available_workspaces = self.get_available_workspaces(&conn)?;

        Ok(DashboardSummary {
            // Row 1
            succeeded_jobs: TrendValue { current: current.succeeded_jobs, previous: previous.succeeded_jobs },
            total_tokens: TrendValue { current: current.total_tokens, previous: previous.total_tokens },
            total_cost: TrendValue { current: current.total_cost, previous: previous.total_cost },
            total_bytes: TrendValue { current: current.total_bytes, previous: previous.total_bytes },
            avg_duration_ms: TrendValue { current: current.avg_duration, previous: previous.avg_duration },
            wall_clock_ms: TrendValue { current: current.wall_clock, previous: previous.wall_clock },
            // Row 2
            input_tokens: TrendValue { current: current.input_tokens, previous: previous.input_tokens },
            output_tokens: TrendValue { current: current.output_tokens, previous: previous.output_tokens },
            cached_tokens: TrendValue { current: current.cached_tokens, previous: previous.cached_tokens },
            total_tool_calls: TrendValue { current: current_tools as f64, previous: previous_tools as f64 },
            total_file_accesses: TrendValue { current: current_files as f64, previous: previous_files as f64 },
            failed_jobs: TrendValue { current: current.failed_jobs, previous: previous.failed_jobs },
            // Breakdowns
            tokens,
            agents,
            modes,
            top_tools,
            top_files,
            available_agents,
            available_modes,
            available_workspaces,
        })
    }

    fn cutoff_days_with_prev(&self, range: TimeRange) -> (Option<String>, Option<String>) {
        match range.days() {
            Some(days) => {
                let now = Utc::now().timestamp_millis();
                let cutoff_ms = now - (days as i64 * 24 * 60 * 60 * 1000);
                let prev_cutoff_ms = cutoff_ms - (days as i64 * 24 * 60 * 60 * 1000);
                (Some(day_bucket(cutoff_ms)), Some(day_bucket(prev_cutoff_ms)))
            }
            None => (None, None),
        }
    }

    fn build_where_clause(&self, filter: &DashboardFilter, cutoff: Option<&str>) -> String {
        fn escape_sql_literal(value: &str) -> String {
            value.replace('\'', "''")
        }

        let mut conditions = Vec::new();
        if let Some(c) = cutoff {
            conditions.push(format!("day_bucket >= '{}'", escape_sql_literal(c)));
        }
        if let Some(agent) = &filter.agent {
            conditions.push(format!("agent_type = '{}'", escape_sql_literal(agent)));
        }
        if let Some(mode) = &filter.mode_or_chain {
            conditions.push(format!("mode = '{}'", escape_sql_literal(mode)));
        }
        if let Some(workspace) = &filter.workspace {
            conditions.push(format!(
                "workspace_path = '{}'",
                escape_sql_literal(workspace)
            ));
        }
        if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        }
    }

    /// Extended period stats tuple:
    /// (succeeded_jobs, total_tokens, total_cost, total_bytes, avg_duration, wall_clock,
    ///  input_tokens, output_tokens, cached_tokens, failed_jobs)
    fn query_period_stats(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<PeriodStats> {
        let sql = format!(
            "SELECT
                COALESCE(SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(input_tokens + output_tokens), 0),
                COALESCE(SUM(cost_usd), 0),
                COALESCE(SUM(input_tokens + output_tokens), 0) * 4,
                COALESCE(AVG(duration_ms), 0),
                COALESCE(MAX(finished_at) - MIN(started_at), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read_tokens + cache_write_tokens), 0),
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0)
             FROM job_stats {}",
            where_clause
        );
        let result = conn.query_row(&sql, [], |row| {
            Ok(PeriodStats {
                succeeded_jobs: row.get(0)?,
                total_tokens: row.get(1)?,
                total_cost: row.get(2)?,
                total_bytes: row.get(3)?,
                avg_duration: row.get(4)?,
                wall_clock: row.get(5)?,
                input_tokens: row.get(6)?,
                output_tokens: row.get(7)?,
                cached_tokens: row.get(8)?,
                failed_jobs: row.get(9)?,
            })
        })?;
        Ok(result)
    }

    fn query_total_tool_calls(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<u64> {
        // Tool stats don't have all job_stats filters, so we join
        let sql = if where_clause.is_empty() {
            "SELECT COUNT(*) FROM tool_stats".to_string()
        } else {
            format!(
                "SELECT COUNT(*) FROM tool_stats t
                 INNER JOIN job_stats j ON t.job_id = j.job_id {}",
                where_clause
            )
        };
        Ok(conn.query_row(&sql, [], |row| row.get(0))?)
    }

    fn query_total_file_accesses(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<u64> {
        let sql = if where_clause.is_empty() {
            "SELECT COUNT(*) FROM file_stats".to_string()
        } else {
            format!(
                "SELECT COUNT(*) FROM file_stats f
                 INNER JOIN job_stats j ON f.job_id = j.job_id {}",
                where_clause
            )
        };
        Ok(conn.query_row(&sql, [], |row| row.get(0))?)
    }

    fn get_available_workspaces(&self, conn: &rusqlite::Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT workspace_path FROM job_stats WHERE workspace_path IS NOT NULL ORDER BY workspace_path"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn query_token_breakdown(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<TokenBreakdown> {
        let sql = format!(
            "SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(cache_write_tokens), 0)
             FROM job_stats {}",
            where_clause
        );
        let result = conn.query_row(&sql, [], |row| {
            Ok(TokenBreakdown {
                input: row.get(0)?,
                output: row.get(1)?,
                cache_read: row.get(2)?,
                cache_write: row.get(3)?,
            })
        })?;
        Ok(result)
    }

    fn query_agent_stats(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<Vec<AgentStats>> {
        let base_where = if where_clause.is_empty() { "" } else { &where_clause[6..] }; // Strip "WHERE "
        let sql = if where_clause.is_empty() {
            "SELECT agent_type,
                    COUNT(*),
                    SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END),
                    COALESCE(SUM(cost_usd), 0),
                    COALESCE(SUM(input_tokens), 0),
                    COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cache_read_tokens), 0),
                    COALESCE(SUM(cache_write_tokens), 0),
                    COALESCE(AVG(duration_ms), 0)
             FROM job_stats GROUP BY agent_type".to_string()
        } else {
            format!(
                "SELECT agent_type,
                        COUNT(*),
                        SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END),
                        COALESCE(SUM(cost_usd), 0),
                        COALESCE(SUM(input_tokens), 0),
                        COALESCE(SUM(output_tokens), 0),
                        COALESCE(SUM(cache_read_tokens), 0),
                        COALESCE(SUM(cache_write_tokens), 0),
                        COALESCE(AVG(duration_ms), 0)
                 FROM job_stats WHERE {} GROUP BY agent_type",
                base_where
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(AgentStats {
                name: row.get(0)?,
                jobs: row.get(1)?,
                succeeded_jobs: row.get(2)?,
                cost_usd: row.get(3)?,
                tokens: TokenBreakdown {
                    input: row.get(4)?,
                    output: row.get(5)?,
                    cache_read: row.get(6)?,
                    cache_write: row.get(7)?,
                },
                avg_duration_ms: row.get::<_, f64>(8)? as u64,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn query_mode_stats(&self, conn: &rusqlite::Connection, where_clause: &str) -> Result<Vec<ModeChainStats>> {
        let base_where = if where_clause.is_empty() { "" } else { &where_clause[6..] };
        let sql = if where_clause.is_empty() {
            "SELECT mode,
                    COUNT(*),
                    SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END),
                    (SELECT agent_type FROM job_stats j2 WHERE j2.mode = job_stats.mode
                     GROUP BY agent_type ORDER BY COUNT(*) DESC LIMIT 1),
                    COALESCE(AVG(cost_usd), 0),
                    COALESCE(AVG(duration_ms), 0),
                    COALESCE(SUM(input_tokens), 0),
                    COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cache_read_tokens + cache_write_tokens), 0),
                    MAX(created_at)
             FROM job_stats GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 20".to_string()
        } else {
            format!(
                "SELECT mode,
                        COUNT(*),
                        SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END),
                        (SELECT agent_type FROM job_stats j2 WHERE j2.mode = job_stats.mode
                         GROUP BY agent_type ORDER BY COUNT(*) DESC LIMIT 1),
                        COALESCE(AVG(cost_usd), 0),
                        COALESCE(AVG(duration_ms), 0),
                        COALESCE(SUM(input_tokens), 0),
                        COALESCE(SUM(output_tokens), 0),
                        COALESCE(SUM(cache_read_tokens + cache_write_tokens), 0),
                        MAX(created_at)
                 FROM job_stats WHERE {} GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 20",
                base_where
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(ModeChainStats {
                name: row.get(0)?,
                total_jobs: row.get(1)?,
                succeeded_jobs: row.get(2)?,
                primary_agent: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                avg_cost_usd: row.get(4)?,
                avg_duration_ms: row.get::<_, f64>(5)? as u64,
                tokens: TokenBreakdown {
                    input: row.get(6)?,
                    output: row.get(7)?,
                    cache_read: row.get::<_, u64>(8)? / 2, // Approximate split
                    cache_write: row.get::<_, u64>(8)? / 2,
                },
                last_used: row.get(9)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn get_available_agents(&self, conn: &rusqlite::Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT DISTINCT agent_type FROM job_stats ORDER BY agent_type")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn get_available_modes(&self, conn: &rusqlite::Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT DISTINCT mode FROM job_stats ORDER BY mode")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
