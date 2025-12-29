//! Dashboard query helper methods

use anyhow::Result;

use crate::stats::models::{AgentStats, ModeChainStats, TokenBreakdown};
use crate::stats::queries::{PeriodStats, StatsQuery};

impl StatsQuery {
    /// Extended period stats query
    pub(super) fn query_period_stats(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<PeriodStats> {
        let sql = format!(
            "SELECT
                COALESCE(SUM(CASE WHEN status IN ('done', 'merged') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens), 0),
                COALESCE(SUM(cost_usd), 0),
                COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens), 0) * 4,
                COALESCE(AVG(duration_ms), 0),
                COALESCE(SUM(duration_ms), 0),
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
                total_duration: row.get(5)?,
                wall_clock: row.get(6)?,
                input_tokens: row.get(7)?,
                output_tokens: row.get(8)?,
                cached_tokens: row.get(9)?,
                failed_jobs: row.get(10)?,
            })
        })?;
        Ok(result)
    }

    pub(super) fn query_total_tool_calls(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<u64> {
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

    pub(super) fn query_total_file_accesses(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<u64> {
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

    pub(super) fn query_top_tools_filtered(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
        limit: usize,
    ) -> Result<Vec<(String, u64)>> {
        let sql = if where_clause.is_empty() {
            "SELECT tool_name, COUNT(*) as cnt
             FROM tool_stats
             GROUP BY tool_name
             ORDER BY cnt DESC
             LIMIT ?"
                .to_string()
        } else {
            format!(
                "SELECT t.tool_name, COUNT(*) as cnt
                 FROM tool_stats t
                 INNER JOIN job_stats j ON t.job_id = j.job_id
                 {}
                 GROUP BY t.tool_name
                 ORDER BY cnt DESC
                 LIMIT ?",
                where_clause
            )
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(super) fn query_top_files_filtered(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
        limit: usize,
    ) -> Result<Vec<(String, u64)>> {
        let sql = if where_clause.is_empty() {
            "SELECT file_path, COUNT(*) as cnt
             FROM file_stats
             GROUP BY file_path
             ORDER BY cnt DESC
             LIMIT ?"
                .to_string()
        } else {
            format!(
                "SELECT f.file_path, COUNT(*) as cnt
                 FROM file_stats f
                 INNER JOIN job_stats j ON f.job_id = j.job_id
                 {}
                 GROUP BY f.file_path
                 ORDER BY cnt DESC
                 LIMIT ?",
                where_clause
            )
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(super) fn get_available_workspaces(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT workspace_path FROM job_stats WHERE workspace_path IS NOT NULL ORDER BY workspace_path"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub(super) fn query_token_breakdown(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<TokenBreakdown> {
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

    pub(super) fn query_agent_stats(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<Vec<AgentStats>> {
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

    pub(super) fn query_mode_stats(
        &self,
        conn: &rusqlite::Connection,
        where_clause: &str,
    ) -> Result<Vec<ModeChainStats>> {
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

    pub(super) fn get_available_agents(&self, conn: &rusqlite::Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT DISTINCT agent_type FROM job_stats ORDER BY agent_type")?;
        Ok(stmt.query_map([], |row| row.get(0))?.filter_map(|r| r.ok()).collect())
    }

    pub(super) fn get_available_modes(&self, conn: &rusqlite::Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT DISTINCT mode FROM job_stats ORDER BY mode")?;
        Ok(stmt.query_map([], |row| row.get(0))?.filter_map(|r| r.ok()).collect())
    }
}
