//! Data models for statistics tracking
//!
//! These structures represent the data stored in and queried from the stats database.

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
    Last7Days,
    #[default]
    Last30Days,
    Last90Days,
    AllTime,
}

impl TimeRange {
    /// Get the number of days to look back (None for all time)
    pub fn days(&self) -> Option<u32> {
        match self {
            Self::Last7Days => Some(7),
            Self::Last30Days => Some(30),
            Self::Last90Days => Some(90),
            Self::AllTime => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
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

// ============================================================================
// Dashboard V2 Models
// ============================================================================

/// Filter options for the dashboard
#[derive(Debug, Clone, Default)]
pub struct DashboardFilter {
    pub agent: Option<String>,         // None = all, Some("claude") or Some("codex")
    pub mode_or_chain: Option<String>, // None = all
    pub workspace: Option<String>,     // None = all, Some(path) filters by workspace_path
}

/// Token breakdown by type
#[derive(Debug, Clone, Default)]
pub struct TokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl TokenBreakdown {
    pub fn total(&self) -> u64 {
        self.input + self.output
    }

    pub fn total_cache(&self) -> u64 {
        self.cache_read + self.cache_write
    }

    /// Cache hit rate as percentage of total input tokens that came from cache
    /// Formula: cache_read / (cache_read + fresh_input) * 100
    pub fn cache_hit_rate(&self) -> f64 {
        let total_input = self.input + self.cache_read;
        if total_input == 0 {
            0.0
        } else {
            (self.cache_read as f64 / total_input as f64) * 100.0
        }
    }
}

/// Agent statistics for ring chart
#[derive(Debug, Clone, Default)]
pub struct AgentStats {
    pub name: String,
    pub jobs: u64,
    pub succeeded_jobs: u64,
    pub cost_usd: f64,
    pub tokens: TokenBreakdown,
    pub avg_duration_ms: u64,
}

/// Mode/Chain statistics for the table
#[derive(Debug, Clone, Default)]
pub struct ModeChainStats {
    pub name: String,
    pub total_jobs: u64,
    pub succeeded_jobs: u64,
    pub primary_agent: String,      // Most used agent
    pub avg_cost_usd: f64,
    pub avg_duration_ms: u64,
    pub tokens: TokenBreakdown,
    pub last_used: i64,             // Timestamp ms
}

impl ModeChainStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_jobs == 0 {
            100.0
        } else {
            (self.succeeded_jobs as f64 / self.total_jobs as f64) * 100.0
        }
    }
}

/// Trend comparison with previous period
#[derive(Debug, Clone, Default)]
pub struct TrendValue {
    pub current: f64,
    pub previous: f64,
}

impl TrendValue {
    pub fn percent_change(&self) -> f64 {
        if self.previous == 0.0 {
            if self.current > 0.0 { 100.0 } else { 0.0 }
        } else {
            ((self.current - self.previous) / self.previous) * 100.0
        }
    }

    pub fn is_positive(&self) -> bool {
        self.current >= self.previous
    }
}

/// Complete dashboard summary with all metrics
#[derive(Debug, Clone, Default)]
pub struct DashboardSummary {
    // Summary card values (with trends) - Row 1
    pub succeeded_jobs: TrendValue,
    pub total_tokens: TrendValue,
    pub total_cost: TrendValue,
    pub total_bytes: TrendValue,
    pub avg_duration_ms: TrendValue,
    /// Wall clock time: max(finished_at) - min(started_at) in milliseconds
    /// This shows actual elapsed time, not sum of job durations
    pub wall_clock_ms: TrendValue,

    // Summary card values - Row 2
    pub input_tokens: TrendValue,
    pub output_tokens: TrendValue,
    pub cached_tokens: TrendValue,
    pub total_tool_calls: TrendValue,
    pub total_file_accesses: TrendValue,
    pub failed_jobs: TrendValue,

    // Token breakdown for ring chart
    pub tokens: TokenBreakdown,

    // Agent breakdown for ring chart
    pub agents: Vec<AgentStats>,

    // Mode/Chain stats for table
    pub modes: Vec<ModeChainStats>,

    // Top tools and files
    pub top_tools: Vec<(String, u64)>,
    pub top_files: Vec<(String, u64)>,

    // Available filter options
    pub available_agents: Vec<String>,
    pub available_modes: Vec<String>,
    pub available_workspaces: Vec<String>,
}

