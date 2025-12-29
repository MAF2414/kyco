//! Statistics tracking module for KYCo
//!
//! Tracks job executions, token usage, tool calls, and file accesses
//! in a SQLite database (`~/.kyco/stats.db`).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐
//! │   Rust GUI      │     │     Bridge      │
//! │   (Job Stats)   │     │  (Tool/File)    │
//! └────────┬────────┘     └────────┬────────┘
//!          │                       │
//!          └───────────┬───────────┘
//!                      ▼
//!             ~/.kyco/stats.db
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let stats = StatsManager::new()?;
//!
//! // Record a completed job
//! stats.recorder().record_job(&job_record)?;
//!
//! // Query for dashboard
//! let summary = stats.query().get_summary(TimeRange::Last30Days)?;
//! ```

mod db;
mod models;
mod queries;
mod recorder;
mod time_bucket;

pub use db::StatsDb;
pub use models::{
    // Legacy exports (kept for compatibility)
    DailyStatsView, FileAccessType, FileStatsRecord, JobStatsRecord, StatsGraph, StatsSummary,
    TimeRange, ToolStatsRecord,
    // Dashboard V2 exports
    AgentStats, DashboardFilter, DashboardSummary, ModeChainStats, TokenBreakdown, TrendValue,
};
pub use queries::StatsQuery;
pub use recorder::StatsRecorder;
pub use time_bucket::{current_day_bucket, current_interval_bucket, day_bucket, interval_bucket};

use anyhow::Result;

/// Central manager for statistics tracking
///
/// Coordinates recording and querying of statistics.
/// Thread-safe through internal mutex on the database connection.
#[derive(Clone)]
pub struct StatsManager {
    db: StatsDb,
}

impl StatsManager {
    /// Create a new StatsManager with the default database location
    pub fn new() -> Result<Self> {
        let db = StatsDb::open_default()?;
        Ok(Self { db })
    }

    /// Create a StatsManager with a custom database path
    pub fn with_path(path: &std::path::Path) -> Result<Self> {
        let db = StatsDb::open(path)?;
        Ok(Self { db })
    }

    /// Get a recorder for writing statistics
    pub fn recorder(&self) -> StatsRecorder {
        StatsRecorder::new(self.db.clone())
    }

    /// Get a query interface for reading statistics
    pub fn query(&self) -> StatsQuery {
        StatsQuery::new(self.db.clone())
    }

    /// Reset all statistics (delete all data)
    pub fn reset_all(&self) -> anyhow::Result<()> {
        self.db.reset_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_stats_manager_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_stats.db");
        let manager = StatsManager::with_path(&db_path).unwrap();

        // Record a job
        let job = JobStatsRecord {
            job_id: 1,
            session_id: Some("test-session".to_string()),
            mode: "refactor".to_string(),
            agent_id: "claude".to_string(),
            agent_type: "claude".to_string(),
            status: "done".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 200,
            cache_write_tokens: 100,
            cost_usd: 0.05,
            duration_ms: 5000,
            files_changed: 3,
            lines_added: 50,
            lines_removed: 20,
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: Some(chrono::Utc::now().timestamp_millis()),
            finished_at: Some(chrono::Utc::now().timestamp_millis()),
            workspace_id: None,
        };

        manager.recorder().record_job(&job).unwrap();

        // Record tool calls
        let tool = ToolStatsRecord {
            job_id: 1,
            session_id: Some("test-session".to_string()),
            tool_name: "Edit".to_string(),
            tool_use_id: Some("tool-1".to_string()),
            success: true,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        manager.recorder().record_tool_call(&tool).unwrap();

        // Record file access
        let file = FileStatsRecord {
            job_id: 1,
            session_id: Some("test-session".to_string()),
            file_path: "src/main.rs".to_string(),
            access_type: FileAccessType::Edit,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        manager.recorder().record_file_access(&file).unwrap();

        // Query summary
        let summary = manager.query().get_summary(TimeRange::AllTime).unwrap();

        assert_eq!(summary.total_jobs, 1);
        assert_eq!(summary.total_tokens, 1500); // 1000 + 500
        assert!((summary.total_cost_usd - 0.05).abs() < 0.001);
        assert!(!summary.top_tools.is_empty());
        assert!(!summary.top_files.is_empty());
    }
}
