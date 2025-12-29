//! Stats query functions for reading aggregated data
//!
//! Provides efficient queries for the Stats GUI view.

mod dashboard;
mod summary;

use super::db::StatsDb;

/// Query interface for statistics
pub struct StatsQuery {
    pub(crate) db: StatsDb,
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

/// Internal struct for period statistics query result
pub(crate) struct PeriodStats {
    pub(crate) succeeded_jobs: f64,
    pub(crate) total_tokens: f64,
    pub(crate) total_cost: f64,
    pub(crate) total_bytes: f64,
    pub(crate) avg_duration: f64,
    pub(crate) total_duration: f64,
    pub(crate) wall_clock: f64,
    pub(crate) input_tokens: f64,
    pub(crate) output_tokens: f64,
    pub(crate) cached_tokens: f64,
    pub(crate) failed_jobs: f64,
}

impl StatsQuery {
    pub fn new(db: StatsDb) -> Self {
        Self { db }
    }
}
