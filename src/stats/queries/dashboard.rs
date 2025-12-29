//! Dashboard V2 queries

mod helpers;

use anyhow::Result;
use chrono::Utc;

use super::{escape_sql_literal, StatsQuery};
use crate::stats::models::{DashboardFilter, DashboardSummary, TimeRange, TrendValue};
use crate::stats::time_bucket::day_bucket;

impl StatsQuery {
    /// Get complete dashboard summary with optional filters
    pub fn get_dashboard(&self, range: TimeRange, filter: &DashboardFilter) -> Result<DashboardSummary> {
        let conn = self.db.conn();
        let (cutoff, prev_cutoff) = self.cutoff_days_with_prev(range);

        // Build WHERE clause based on filter
        let where_clause = self.build_where_clause(filter, cutoff.as_deref());
        let prev_where = self.build_where_clause(filter, prev_cutoff.as_deref());
        let join_where = self.build_where_clause_for_alias(filter, cutoff.as_deref(), "j");
        let prev_join_where = self.build_where_clause_for_alias(filter, prev_cutoff.as_deref(), "j");

        // Get current period stats
        let current = self.query_period_stats(&conn, &where_clause)?;
        let previous = self.query_period_stats(&conn, &prev_where)?;

        // Get tool calls and file accesses
        let current_tools = self.query_total_tool_calls(&conn, &join_where).unwrap_or(0);
        let previous_tools = self.query_total_tool_calls(&conn, &prev_join_where).unwrap_or(0);
        let current_files = self.query_total_file_accesses(&conn, &join_where).unwrap_or(0);
        let previous_files = self.query_total_file_accesses(&conn, &prev_join_where).unwrap_or(0);

        // Get token breakdown
        let tokens = self.query_token_breakdown(&conn, &where_clause)?;

        // Get agent breakdown
        let agents = self.query_agent_stats(&conn, &where_clause)?;

        // Get mode stats
        let modes = self.query_mode_stats(&conn, &where_clause)?;

        // Get top tools/files (filtered to match dashboard)
        let top_tools = self.query_top_tools_filtered(&conn, &join_where, 8)?;
        let top_files = self.query_top_files_filtered(&conn, &join_where, 8)?;

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
            total_duration_ms: TrendValue { current: current.total_duration, previous: previous.total_duration },
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

    fn build_where_clause_for_alias(
        &self,
        filter: &DashboardFilter,
        cutoff: Option<&str>,
        alias: &str,
    ) -> String {
        let mut conditions = Vec::new();
        if let Some(c) = cutoff {
            conditions.push(format!("{alias}.day_bucket >= '{}'", escape_sql_literal(c)));
        }
        if let Some(agent) = &filter.agent {
            conditions.push(format!(
                "{alias}.agent_type = '{}'",
                escape_sql_literal(agent)
            ));
        }
        if let Some(mode) = &filter.mode_or_chain {
            conditions.push(format!("{alias}.mode = '{}'", escape_sql_literal(mode)));
        }
        if let Some(workspace) = &filter.workspace {
            conditions.push(format!(
                "{alias}.workspace_path = '{}'",
                escape_sql_literal(workspace)
            ));
        }
        if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        }
    }
}
