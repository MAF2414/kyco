//! SQLite database connection and schema management for BugBounty
//!
//! Manages the `~/.kyco/bugbounty.db` database with automatic schema migration.
//! Separate from stats.db to keep concerns isolated.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::config::Config;

/// Database wrapper for BugBounty data
#[derive(Clone)]
pub struct BugBountyDb {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl BugBountyDb {
    /// Open or create the bugbounty database at the default location (~/.kyco/bugbounty.db)
    pub fn open_default() -> Result<Self> {
        let db_path = Config::global_config_dir().join("bugbounty.db");
        Self::open(&db_path)
    }

    /// Open or create the bugbounty database at a specific path
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create bugbounty dir: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open bugbounty db: {}", path.display()))?;

        // Enable WAL mode for concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Get a reference to the connection
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("BugBounty DB lock poisoned")
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(SCHEMA_SQL)?;
        drop(conn);
        self.run_migrations()?;
        Ok(())
    }

    /// Run any pending migrations
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn();

        // Get current schema version
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM bb_schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        // Future migrations go here
        // if version < 2 { ... }

        let _ = version; // Suppress unused warning for now

        Ok(())
    }

    /// Delete all bugbounty data (reset to empty state)
    pub fn reset_all(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            r#"
            DELETE FROM flow_edges;
            DELETE FROM artifacts;
            DELETE FROM job_findings;
            DELETE FROM findings;
            DELETE FROM jobs;
            DELETE FROM projects;
            "#,
        )?;
        Ok(())
    }
}

/// SQL schema for the bugbounty database
const SCHEMA_SQL: &str = r#"
-- Schema version tracking (prefixed to avoid collision with other dbs)
CREATE TABLE IF NOT EXISTS bb_schema_version (version INTEGER PRIMARY KEY);
INSERT OR IGNORE INTO bb_schema_version VALUES (1);

-- ============================================
-- PROJECTS (BugBounty Programs)
-- ============================================
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,                    -- e.g., "hackerone-nextcloud"
    root_path TEXT NOT NULL,                -- e.g., "BugBounty/programs/hackerone-nextcloud/"
    platform TEXT,                          -- hackerone, intigriti, bugcrowd
    target_name TEXT,                       -- nextcloud, miro, etc.
    scope_json TEXT,                        -- Parsed scope.md as JSON
    tool_policy_json TEXT,                  -- Tool restrictions
    metadata_json TEXT,                     -- Stack, auth notes, etc.
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);
CREATE INDEX IF NOT EXISTS idx_projects_platform ON projects(platform);

-- ============================================
-- FINDINGS (Kanban Cards)
-- ============================================
CREATE TABLE IF NOT EXISTS findings (
    id TEXT PRIMARY KEY,                    -- VULN-001, VULN-002, etc.
    project_id TEXT NOT NULL,
    title TEXT NOT NULL,
    severity TEXT,                          -- critical, high, medium, low, info
    status TEXT DEFAULT 'raw',              -- raw, needs_repro, verified, report_draft, submitted, etc.

    -- Structured output fields (from security-audit profile)
    attack_scenario TEXT,
    preconditions TEXT,
    reachability TEXT,                      -- public, auth_required, internal_only
    impact TEXT,
    confidence TEXT,                        -- high, medium, low

    -- Optional fields
    cwe_id TEXT,
    cvss_score REAL,
    affected_assets_json TEXT,              -- JSON array of affected endpoints/domains
    taint_path TEXT,                        -- Entry -> ... -> Sink

    -- Metadata
    fp_reason TEXT,                         -- If marked false positive
    notes TEXT,
    source_file TEXT,                       -- Path to notes/findings/VULN-XXX.md

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_findings_project ON findings(project_id);
CREATE INDEX IF NOT EXISTS idx_findings_status ON findings(status);
CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);

-- ============================================
-- JOBS (extended with project/finding links)
-- ============================================
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    kyco_job_id INTEGER,                    -- Link to KYCo's internal job ID
    mode TEXT,
    target_files_json TEXT,                 -- JSON array of files
    prompt TEXT,

    status TEXT DEFAULT 'pending',          -- pending, running, done, failed
    result_state TEXT,                      -- Agent's result.state
    next_context_json TEXT,                 -- Agent's next_context output

    started_at INTEGER,
    completed_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_jobs_project ON jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_kyco_id ON jobs(kyco_job_id);

-- ============================================
-- JOB <-> FINDING LINKS
-- ============================================
CREATE TABLE IF NOT EXISTS job_findings (
    job_id TEXT NOT NULL,
    finding_id TEXT NOT NULL,
    link_type TEXT DEFAULT 'discovered',    -- discovered, verified, investigated
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),

    PRIMARY KEY (job_id, finding_id),
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);

-- ============================================
-- ARTIFACTS (Evidence)
-- ============================================
CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,
    finding_id TEXT,
    job_id TEXT,

    type TEXT NOT NULL,                     -- http_request, http_response, screenshot, log, poc_file
    path TEXT NOT NULL,                     -- Relative to project root
    description TEXT,
    hash TEXT,                              -- For deduplication

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),

    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_artifacts_finding ON artifacts(finding_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_job ON artifacts(job_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_type ON artifacts(type);

-- ============================================
-- FLOW EDGES (Taint Tracking)
-- ============================================
CREATE TABLE IF NOT EXISTS flow_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    finding_id TEXT NOT NULL,

    from_file TEXT,
    from_line INTEGER,
    from_symbol TEXT,

    to_file TEXT,
    to_line INTEGER,
    to_symbol TEXT,

    kind TEXT,                              -- taint, authz, dataflow, controlflow
    notes TEXT,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),

    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_flow_edges_finding ON flow_edges(finding_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_and_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        let db = BugBountyDb::open(&db_path).unwrap();

        // Verify tables exist
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"findings".to_string()));
        assert!(tables.contains(&"jobs".to_string()));
        assert!(tables.contains(&"artifacts".to_string()));
        assert!(tables.contains(&"flow_edges".to_string()));
    }

    #[test]
    fn test_reset_all() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        let db = BugBountyDb::open(&db_path).unwrap();

        // Insert a project
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO projects (id, root_path, platform) VALUES ('test', '/tmp/test', 'hackerone')",
                [],
            )
            .unwrap();
        }

        // Reset
        db.reset_all().unwrap();

        // Verify empty
        let conn = db.conn();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
