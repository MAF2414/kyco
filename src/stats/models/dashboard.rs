//! Dashboard V2 data models
//!
//! These structures represent the dashboard summary and filter data.

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
    /// Total time: sum(duration_ms) in milliseconds
    pub total_duration_ms: TrendValue,
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
