//! Data models for statistics tracking
//!
//! These structures represent the data stored in and queried from the stats database.

mod dashboard;

pub use dashboard::{
    AgentStats, DashboardFilter, DashboardSummary, ModeChainStats, TokenBreakdown, TrendValue,
};

use serde::{Deserialize, Serialize};

/// Record for a completed job's statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatsRecord {
    pub job_id: u64,
    pub session_id: Option<String>,
    pub mode: String,
    pub agent_id: String,
    pub agent_type: String, // "claude" or "codex"
    pub status: String,     // "done", "failed", "merged", etc.

    // Token usage
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,

    // Cost and duration
    pub cost_usd: f64,
    pub duration_ms: u64,

    // File changes
    pub files_changed: usize,
    pub lines_added: usize,
    pub lines_removed: usize,

    // Timestamps (ms since epoch)
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,

    pub workspace_path: Option<String>,
}

/// Record for a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatsRecord {
    pub job_id: u64,
    pub session_id: Option<String>,
    pub tool_name: String,
    pub tool_use_id: Option<String>,
    pub success: bool,
    pub timestamp: i64,
}

/// Type of file access operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAccessType {
    Read,
    Write,
    Edit,
}

impl FileAccessType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Edit => "edit",
        }
    }
}

/// Record for a file access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatsRecord {
    pub job_id: u64,
    pub session_id: Option<String>,
    pub file_path: String,
    pub access_type: FileAccessType,
    pub timestamp: i64,
}

/// Aggregated daily statistics for display
#[derive(Debug, Clone, Default)]
pub struct DailyStatsView {
    pub day: String, // YYYY-MM-DD
    pub total_jobs: u64,
    pub done_jobs: u64,
    pub failed_jobs: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub claude_jobs: u64,
    pub codex_jobs: u64,
    pub total_tool_calls: u64,
}

/// Summary statistics for the dashboard
#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    // Overall totals
    pub total_jobs: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub total_tool_calls: u64,

    // By status (name, count)
    pub jobs_by_status: Vec<(String, u64)>,

    // By agent (name, job_count, token_count)
    pub jobs_by_agent: Vec<(String, u64, u64)>,

    // By mode (name, count)
    pub top_modes: Vec<(String, u64)>,

    // Top tools (name, count)
    pub top_tools: Vec<(String, u64)>,

    // Top files (path, count)
    pub top_files: Vec<(String, u64)>,

    // Time series (last N days)
    pub daily_stats: Vec<DailyStatsView>,
}

impl StatsSummary {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let done = self
            .jobs_by_status
            .iter()
            .find(|(s, _)| s == "done" || s == "merged")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let failed = self
            .jobs_by_status
            .iter()
            .find(|(s, _)| s == "failed")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let total = done + failed;
        if total == 0 {
            100.0
        } else {
            (done as f64 / total as f64) * 100.0
        }
    }
}

/// Time range for filtering stats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeRange {
    Last15Minutes,
    Last30Minutes,
    Last1Hour,
    Last8Hours,
    Last1Day,
    Last3Days,
    Last7Days,
    #[default]
    Last30Days,
    Last90Days,
    AllTime,
}

impl TimeRange {
    /// Get the number of days to look back (None for all time).
    ///
    /// For sub-day ranges, this returns 1 to include the current day bucket.
    pub fn days(&self) -> Option<u32> {
        match self {
            Self::Last15Minutes => Some(1),
            Self::Last30Minutes => Some(1),
            Self::Last1Hour => Some(1),
            Self::Last8Hours => Some(1),
            Self::Last1Day => Some(1),
            Self::Last3Days => Some(3),
            Self::Last7Days => Some(7),
            Self::Last30Days => Some(30),
            Self::Last90Days => Some(90),
            Self::AllTime => None,
        }
    }

    /// Get the lookback window in milliseconds (None for all time).
    pub fn window_ms(&self) -> Option<i64> {
        const MINUTE_MS: i64 = 60 * 1000;
        const HOUR_MS: i64 = 60 * MINUTE_MS;
        const DAY_MS: i64 = 24 * HOUR_MS;

        match self {
            Self::Last15Minutes => Some(15 * MINUTE_MS),
            Self::Last30Minutes => Some(30 * MINUTE_MS),
            Self::Last1Hour => Some(1 * HOUR_MS),
            Self::Last8Hours => Some(8 * HOUR_MS),
            Self::Last1Day => Some(1 * DAY_MS),
            Self::Last3Days => Some(3 * DAY_MS),
            Self::Last7Days => Some(7 * DAY_MS),
            Self::Last30Days => Some(30 * DAY_MS),
            Self::Last90Days => Some(90 * DAY_MS),
            Self::AllTime => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Last15Minutes => "Last 15 min",
            Self::Last30Minutes => "Last 30 min",
            Self::Last1Hour => "Last 1 hour",
            Self::Last8Hours => "Last 8 hours",
            Self::Last1Day => "Last 1 day",
            Self::Last3Days => "Last 3 days",
            Self::Last7Days => "Last 7 days",
            Self::Last30Days => "Last 30 days",
            Self::Last90Days => "Last 90 days",
            Self::AllTime => "All time",
        }
    }
}

/// Which graph to display in the stats view (legacy, kept for compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsGraph {
    #[default]
    JobsOverTime,
    TokensOverTime,
    CostOverTime,
    ModeUsage,
    AgentComparison,
}
