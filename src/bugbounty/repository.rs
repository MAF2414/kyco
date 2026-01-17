//! Repository implementations for BugBounty data access

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

use super::db::BugBountyDb;
use super::models::{
    Artifact, ArtifactType, BugBountyJob, CodeLocation, Confidence, Finding, FindingStatus,
    FlowEdge, FlowKind, FlowTrace, MemoryConfidence, MemoryLocation, MemorySourceKind, MemoryType,
    Project, ProjectMemory, Reachability, Severity,
};

// ============================================
// PROJECT REPOSITORY
// ============================================

/// Repository for Project CRUD operations
pub struct ProjectRepository {
    db: BugBountyDb,
}

/// Aggregated per-project stats (SQL computed).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectStats {
    pub jobs_total: usize,
    pub jobs_pending: usize,
    pub jobs_running: usize,
    pub jobs_done: usize,
    pub jobs_failed: usize,
    pub last_job_at: Option<i64>,
    pub findings_total: usize,
    pub findings_raw: usize,
    pub findings_verified: usize,
    pub findings_submitted: usize,
    pub findings_terminal: usize,
    pub last_finding_at: Option<i64>,
    pub last_activity_at: Option<i64>,
}

impl ProjectRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    /// Create a new project
    pub fn create(&self, project: &Project) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO projects (id, root_path, platform, target_name, scope_json, tool_policy_json, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                project.id,
                project.root_path,
                project.platform,
                project.target_name,
                project.scope.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                project.tool_policy.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                project.metadata.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                project.created_at,
                project.updated_at,
            ],
        ).context("Failed to create project")?;
        Ok(())
    }

    /// Get a project by ID
    pub fn get(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, root_path, platform, target_name, scope_json, tool_policy_json, metadata_json, created_at, updated_at
            FROM projects WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Project {
                id: row.get(0)?,
                root_path: row.get(1)?,
                platform: row.get(2)?,
                target_name: row.get(3)?,
                scope: row.get::<_, Option<String>>(4)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                tool_policy: row.get::<_, Option<String>>(5)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row.get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        });

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all projects
    pub fn list(&self) -> Result<Vec<Project>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, root_path, platform, target_name, scope_json, tool_policy_json, metadata_json, created_at, updated_at
            FROM projects ORDER BY updated_at DESC
            "#,
        )?;

        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                root_path: row.get(1)?,
                platform: row.get(2)?,
                target_name: row.get(3)?,
                scope: row.get::<_, Option<String>>(4)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                tool_policy: row.get::<_, Option<String>>(5)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row.get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(projects)
    }

    /// Update a project
    pub fn update(&self, project: &Project) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            UPDATE projects SET
                root_path = ?2,
                platform = ?3,
                target_name = ?4,
                scope_json = ?5,
                tool_policy_json = ?6,
                metadata_json = ?7,
                updated_at = ?8
            WHERE id = ?1
            "#,
            params![
                project.id,
                project.root_path,
                project.platform,
                project.target_name,
                project.scope.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                project.tool_policy.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                project.metadata.as_ref().map(|s| serde_json::to_string(s).ok()).flatten(),
                now,
            ],
        ).context("Failed to update project")?;
        Ok(())
    }

    /// Delete a project
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// List projects by platform
    pub fn list_by_platform(&self, platform: &str) -> Result<Vec<Project>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, root_path, platform, target_name, scope_json, tool_policy_json, metadata_json, created_at, updated_at
            FROM projects WHERE platform = ?1 ORDER BY updated_at DESC
            "#,
        )?;

        let projects = stmt.query_map(params![platform], |row| {
            Ok(Project {
                id: row.get(0)?,
                root_path: row.get(1)?,
                platform: row.get(2)?,
                target_name: row.get(3)?,
                scope: row.get::<_, Option<String>>(4)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                tool_policy: row.get::<_, Option<String>>(5)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                metadata: row.get::<_, Option<String>>(6)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(projects)
    }

    /// Get aggregated stats for a project (counts + last activity).
    ///
    /// This avoids scanning all jobs/findings in Rust for large projects.
    pub fn get_stats(&self, project_id: &str) -> Result<ProjectStats> {
        let conn = self.db.conn();

        let (jobs_total, jobs_pending, jobs_running, jobs_done, jobs_failed, last_job_at): (
            i64,
            i64,
            i64,
            i64,
            i64,
            Option<i64>,
        ) = conn.query_row(
            r#"
            SELECT
                COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) AS pending,
                COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) AS running,
                COALESCE(SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END), 0) AS done,
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) AS failed,
                MAX(COALESCE(completed_at, started_at, created_at)) AS last_job_at
            FROM jobs
            WHERE project_id = ?1
            "#,
            params![project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;

        let (
            findings_total,
            findings_raw,
            findings_verified,
            findings_submitted,
            findings_terminal,
            last_finding_at,
        ): (i64, i64, i64, i64, i64, Option<i64>) = conn.query_row(
            r#"
            SELECT
                COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN status = 'raw' THEN 1 ELSE 0 END), 0) AS raw,
                COALESCE(SUM(CASE WHEN status = 'verified' THEN 1 ELSE 0 END), 0) AS verified,
                COALESCE(SUM(CASE WHEN status IN ('submitted','triaged','accepted','paid') THEN 1 ELSE 0 END), 0) AS submitted,
                COALESCE(SUM(CASE WHEN status IN ('paid','duplicate','wont_fix','false_positive','out_of_scope') THEN 1 ELSE 0 END), 0) AS terminal,
                MAX(CASE WHEN updated_at > created_at THEN updated_at ELSE created_at END) AS last_finding_at
            FROM findings
            WHERE project_id = ?1
            "#,
            params![project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;

        let last_activity_at = match (last_job_at, last_finding_at) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Ok(ProjectStats {
            jobs_total: usize::try_from(jobs_total).unwrap_or(0),
            jobs_pending: usize::try_from(jobs_pending).unwrap_or(0),
            jobs_running: usize::try_from(jobs_running).unwrap_or(0),
            jobs_done: usize::try_from(jobs_done).unwrap_or(0),
            jobs_failed: usize::try_from(jobs_failed).unwrap_or(0),
            last_job_at,
            findings_total: usize::try_from(findings_total).unwrap_or(0),
            findings_raw: usize::try_from(findings_raw).unwrap_or(0),
            findings_verified: usize::try_from(findings_verified).unwrap_or(0),
            findings_submitted: usize::try_from(findings_submitted).unwrap_or(0),
            findings_terminal: usize::try_from(findings_terminal).unwrap_or(0),
            last_finding_at,
            last_activity_at,
        })
    }
}

// ============================================
// FINDING REPOSITORY
// ============================================

/// Repository for Finding CRUD operations
pub struct FindingRepository {
    db: BugBountyDb,
}

impl FindingRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    /// Create a new finding
    pub fn create(&self, finding: &Finding) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO findings (
                id, project_id, title, severity, status,
                attack_scenario, preconditions, reachability, impact, confidence,
                cwe_id, cvss_score, affected_assets_json, taint_path,
                fp_reason, notes, source_file, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19
            )
            "#,
            params![
                finding.id,
                finding.project_id,
                finding.title,
                finding.severity.map(|s| s.as_str()),
                finding.status.as_str(),
                finding.attack_scenario,
                finding.preconditions,
                finding.reachability.map(|r| r.as_str()),
                finding.impact,
                finding.confidence.map(|c| c.as_str()),
                finding.cwe_id,
                finding.cvss_score,
                serde_json::to_string(&finding.affected_assets).ok(),
                finding.taint_path,
                finding.fp_reason,
                finding.notes,
                finding.source_file,
                finding.created_at,
                finding.updated_at,
            ],
        ).context("Failed to create finding")?;
        Ok(())
    }

    /// Get a finding by ID
    pub fn get(&self, id: &str) -> Result<Option<Finding>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, title, severity, status,
                   attack_scenario, preconditions, reachability, impact, confidence,
                   cwe_id, cvss_score, affected_assets_json, taint_path,
                   fp_reason, notes, source_file, created_at, updated_at
            FROM findings WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(self.row_to_finding(row))
        });

        match result {
            Ok(finding) => Ok(Some(finding)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set the status of a finding (Kanban column change)
    pub fn set_status(&self, id: &str, status: FindingStatus) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE findings SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), now],
        ).context("Failed to update finding status")?;
        Ok(())
    }

    /// Mark a finding as false positive with reason
    pub fn mark_false_positive(&self, id: &str, reason: &str) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE findings SET status = 'false_positive', fp_reason = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, reason, now],
        ).context("Failed to mark finding as FP")?;
        Ok(())
    }

    /// List findings by project
    pub fn list_by_project(&self, project_id: &str) -> Result<Vec<Finding>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, title, severity, status,
                   attack_scenario, preconditions, reachability, impact, confidence,
                   cwe_id, cvss_score, affected_assets_json, taint_path,
                   fp_reason, notes, source_file, created_at, updated_at
            FROM findings WHERE project_id = ?1 ORDER BY created_at DESC
            "#,
        )?;

        let findings = stmt.query_map(params![project_id], |row| {
            Ok(self.row_to_finding(row))
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(findings)
    }

    /// List findings by status
    pub fn list_by_status(&self, status: FindingStatus) -> Result<Vec<Finding>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, title, severity, status,
                   attack_scenario, preconditions, reachability, impact, confidence,
                   cwe_id, cvss_score, affected_assets_json, taint_path,
                   fp_reason, notes, source_file, created_at, updated_at
            FROM findings WHERE status = ?1 ORDER BY updated_at DESC
            "#,
        )?;

        let findings = stmt.query_map(params![status.as_str()], |row| {
            Ok(self.row_to_finding(row))
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(findings)
    }

    /// Get the next available finding number for a project
    pub fn next_number(&self, project_id: &str) -> Result<u32> {
        let conn = self.db.conn();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM findings WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;
        Ok((count + 1) as u32)
    }

    /// Update a finding
    pub fn update(&self, finding: &Finding) -> Result<()> {
        let conn = self.db.conn();
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            r#"
            UPDATE findings SET
                title = ?2, severity = ?3, status = ?4,
                attack_scenario = ?5, preconditions = ?6, reachability = ?7, impact = ?8, confidence = ?9,
                cwe_id = ?10, cvss_score = ?11, affected_assets_json = ?12, taint_path = ?13,
                fp_reason = ?14, notes = ?15, source_file = ?16, updated_at = ?17
            WHERE id = ?1
            "#,
            params![
                finding.id,
                finding.title,
                finding.severity.map(|s| s.as_str()),
                finding.status.as_str(),
                finding.attack_scenario,
                finding.preconditions,
                finding.reachability.map(|r| r.as_str()),
                finding.impact,
                finding.confidence.map(|c| c.as_str()),
                finding.cwe_id,
                finding.cvss_score,
                serde_json::to_string(&finding.affected_assets).ok(),
                finding.taint_path,
                finding.fp_reason,
                finding.notes,
                finding.source_file,
                now,
            ],
        ).context("Failed to update finding")?;
        Ok(())
    }

    /// Delete a finding
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM findings WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Helper to convert a row to a Finding
    fn row_to_finding(&self, row: &rusqlite::Row) -> Finding {
        Finding {
            id: row.get(0).unwrap_or_default(),
            project_id: row.get(1).unwrap_or_default(),
            title: row.get(2).unwrap_or_default(),
            severity: row.get::<_, Option<String>>(3).ok().flatten()
                .and_then(|s| Severity::from_str(&s)),
            status: row.get::<_, String>(4).ok()
                .and_then(|s| FindingStatus::from_str(&s))
                .unwrap_or(FindingStatus::Raw),
            attack_scenario: row.get(5).ok().flatten(),
            preconditions: row.get(6).ok().flatten(),
            reachability: row.get::<_, Option<String>>(7).ok().flatten()
                .and_then(|s| Reachability::from_str(&s)),
            impact: row.get(8).ok().flatten(),
            confidence: row.get::<_, Option<String>>(9).ok().flatten()
                .and_then(|s| Confidence::from_str(&s)),
            cwe_id: row.get(10).ok().flatten(),
            cvss_score: row.get(11).ok().flatten(),
            affected_assets: row.get::<_, Option<String>>(12).ok().flatten()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            taint_path: row.get(13).ok().flatten(),
            fp_reason: row.get(14).ok().flatten(),
            notes: row.get(15).ok().flatten(),
            source_file: row.get(16).ok().flatten(),
            created_at: row.get(17).unwrap_or(0),
            updated_at: row.get(18).unwrap_or(0),
        }
    }
}

// ============================================
// ARTIFACT REPOSITORY
// ============================================

/// Repository for Artifact CRUD operations
pub struct ArtifactRepository {
    db: BugBountyDb,
}

impl ArtifactRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    /// Create a new artifact
    pub fn create(&self, artifact: &Artifact) -> Result<()> {
        let conn = self.db.conn();

        // Best-effort dedupe: if a hash is provided, avoid inserting duplicates for the same
        // finding (preferred) or job (fallback).
        if let Some(hash) = artifact
            .hash
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let existing: Option<String> = if let Some(ref fid) = artifact.finding_id {
                conn.query_row(
                    "SELECT id FROM artifacts WHERE finding_id = ?1 AND hash = ?2 LIMIT 1",
                    params![fid, hash],
                    |row| row.get(0),
                )
                .optional()?
            } else if let Some(ref jid) = artifact.job_id {
                conn.query_row(
                    "SELECT id FROM artifacts WHERE job_id = ?1 AND hash = ?2 LIMIT 1",
                    params![jid, hash],
                    |row| row.get(0),
                )
                .optional()?
            } else {
                conn.query_row(
                    "SELECT id FROM artifacts WHERE hash = ?1 LIMIT 1",
                    params![hash],
                    |row| row.get(0),
                )
                .optional()?
            };

            if existing.is_some() {
                return Ok(());
            }
        }

        conn.execute(
            r#"
            INSERT INTO artifacts (id, finding_id, job_id, type, path, description, hash, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                artifact.id,
                artifact.finding_id,
                artifact.job_id,
                artifact.artifact_type.as_str(),
                artifact.path,
                artifact.description,
                artifact.hash,
                artifact.created_at,
            ],
        ).context("Failed to create artifact")?;
        Ok(())
    }

    /// Get an artifact by ID
    pub fn get(&self, id: &str) -> Result<Option<Artifact>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, finding_id, job_id, type, path, description, hash, created_at FROM artifacts WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                job_id: row.get(2)?,
                artifact_type: row.get::<_, String>(3)
                    .map(|s| ArtifactType::from_str(&s).unwrap_or(ArtifactType::Other))?,
                path: row.get(4)?,
                description: row.get(5)?,
                hash: row.get(6)?,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(artifact) => Ok(Some(artifact)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List artifacts by finding
    pub fn list_by_finding(&self, finding_id: &str) -> Result<Vec<Artifact>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, finding_id, job_id, type, path, description, hash, created_at FROM artifacts WHERE finding_id = ?1 ORDER BY created_at",
        )?;

        let artifacts = stmt.query_map(params![finding_id], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                job_id: row.get(2)?,
                artifact_type: row.get::<_, String>(3)
                    .map(|s| ArtifactType::from_str(&s).unwrap_or(ArtifactType::Other))?,
                path: row.get(4)?,
                description: row.get(5)?,
                hash: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(artifacts)
    }

    /// List artifacts by job
    pub fn list_by_job(&self, job_id: &str) -> Result<Vec<Artifact>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, finding_id, job_id, type, path, description, hash, created_at FROM artifacts WHERE job_id = ?1 ORDER BY created_at",
        )?;

        let artifacts = stmt.query_map(params![job_id], |row| {
            Ok(Artifact {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                job_id: row.get(2)?,
                artifact_type: row.get::<_, String>(3)
                    .map(|s| ArtifactType::from_str(&s).unwrap_or(ArtifactType::Other))?,
                path: row.get(4)?,
                description: row.get(5)?,
                hash: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(artifacts)
    }

    /// Delete an artifact
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM artifacts WHERE id = ?1", params![id])?;
        Ok(())
    }
}

// ============================================
// FLOW EDGE REPOSITORY
// ============================================

/// Repository for FlowEdge CRUD operations
pub struct FlowEdgeRepository {
    db: BugBountyDb,
}

impl FlowEdgeRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    /// Create a new flow edge
    pub fn create(&self, edge: &FlowEdge) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO flow_edges (finding_id, from_file, from_line, from_symbol, to_file, to_line, to_symbol, kind, notes, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                edge.finding_id,
                edge.from.file,
                edge.from.line,
                edge.from.symbol,
                edge.to.file,
                edge.to.line,
                edge.to.symbol,
                edge.kind.as_str(),
                edge.notes,
                edge.created_at,
            ],
        ).context("Failed to create flow edge")?;
        Ok(())
    }

    /// Get all flow edges for a finding as a FlowTrace
    pub fn get_trace(&self, finding_id: &str) -> Result<FlowTrace> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, finding_id, from_file, from_line, from_symbol, to_file, to_line, to_symbol, kind, notes, created_at
            FROM flow_edges WHERE finding_id = ?1 ORDER BY id
            "#,
        )?;

        let edges: Vec<FlowEdge> = stmt.query_map(params![finding_id], |row| {
            Ok(FlowEdge {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                from: CodeLocation {
                    file: row.get(2)?,
                    line: row.get(3)?,
                    column: None, // Not stored in current schema
                    symbol: row.get(4)?,
                    snippet: None, // Not stored in current schema
                },
                to: CodeLocation {
                    file: row.get(5)?,
                    line: row.get(6)?,
                    column: None, // Not stored in current schema
                    symbol: row.get(7)?,
                    snippet: None, // Not stored in current schema
                },
                kind: row.get::<_, String>(8)
                    .map(|s| FlowKind::from_str(&s).unwrap_or(FlowKind::Dataflow))?,
                notes: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        let mut trace = FlowTrace::new(finding_id);
        for edge in edges {
            trace.add_edge(edge);
        }

        Ok(trace)
    }

    /// Delete all flow edges for a finding
    pub fn delete_for_finding(&self, finding_id: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM flow_edges WHERE finding_id = ?1", params![finding_id])?;
        Ok(())
    }
}

// ============================================
// JOB REPOSITORY
// ============================================

/// Repository for BugBounty job persistence
pub struct JobRepository {
    db: BugBountyDb,
}

impl JobRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    fn row_to_job(&self, row: &rusqlite::Row<'_>) -> BugBountyJob {
        let target_files: Vec<String> = row
            .get::<_, Option<String>>(4)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let next_context: Option<serde_json::Value> = row
            .get::<_, Option<String>>(8)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok());

        BugBountyJob {
            id: row.get(0).unwrap_or_default(),
            project_id: row.get(1).ok().flatten(),
            kyco_job_id: row
                .get::<_, Option<i64>>(2)
                .ok()
                .flatten()
                .and_then(|v| u64::try_from(v).ok()),
            mode: row.get(3).ok().flatten(),
            target_files,
            prompt: row.get(5).ok().flatten(),
            status: row.get(6).unwrap_or_else(|_| "pending".to_string()),
            result_state: row.get(7).ok().flatten(),
            next_context,
            started_at: row.get(9).ok().flatten(),
            completed_at: row.get(10).ok().flatten(),
            created_at: row.get(11).unwrap_or(0),
        }
    }

    /// Create or update a job record
    pub fn upsert(&self, job: &BugBountyJob) -> Result<()> {
        let conn = self.db.conn();

        let target_files_json = if job.target_files.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&job.target_files)?)
        };

        let next_context_json = job
            .next_context
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let kyco_job_id: Option<i64> = job.kyco_job_id.map(|v| v as i64);

        conn.execute(
            r#"
            INSERT INTO jobs (
                id, project_id, kyco_job_id, mode, target_files_json, prompt,
                status, result_state, next_context_json, started_at, completed_at, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12
            )
            ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                kyco_job_id = excluded.kyco_job_id,
                mode = excluded.mode,
                target_files_json = excluded.target_files_json,
                prompt = excluded.prompt,
                status = excluded.status,
                result_state = excluded.result_state,
                next_context_json = excluded.next_context_json,
                started_at = COALESCE(jobs.started_at, excluded.started_at),
                completed_at = excluded.completed_at
            "#,
            params![
                job.id,
                job.project_id,
                kyco_job_id,
                job.mode,
                target_files_json,
                job.prompt,
                job.status,
                job.result_state,
                next_context_json,
                job.started_at,
                job.completed_at,
                job.created_at,
            ],
        )
        .context("Failed to upsert job")?;

        Ok(())
    }

    /// Mark a job as running and set started_at if missing.
    pub fn mark_running(&self, id: &str, started_at: i64) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE jobs SET status = 'running', started_at = COALESCE(started_at, ?2) WHERE id = ?1",
            params![id, started_at],
        )
        .context("Failed to mark job running")?;
        Ok(())
    }

    /// Mark a job as completed (done/failed) and store result fields.
    pub fn mark_completed(
        &self,
        id: &str,
        status: &str,
        completed_at: i64,
        result_state: Option<&str>,
        next_context: Option<&serde_json::Value>,
    ) -> Result<()> {
        let conn = self.db.conn();
        let next_context_json = next_context.map(serde_json::to_string).transpose()?;
        conn.execute(
            r#"
            UPDATE jobs SET
                status = ?2,
                result_state = ?3,
                next_context_json = ?4,
                completed_at = ?5
            WHERE id = ?1
            "#,
            params![id, status, result_state, next_context_json, completed_at],
        )
        .context("Failed to mark job completed")?;
        Ok(())
    }

    /// Ensure a job row exists (needed for artifact FK constraints).
    pub fn ensure_exists(&self, id: &str, project_id: Option<&str>) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR IGNORE INTO jobs (id, project_id, status) VALUES (?1, ?2, 'pending')",
            params![id, project_id],
        )?;

        if let Some(project_id) = project_id {
            conn.execute(
                "UPDATE jobs SET project_id = COALESCE(project_id, ?2) WHERE id = ?1",
                params![id, project_id],
            )?;
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<BugBountyJob>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, kyco_job_id, mode, target_files_json, prompt,
                   status, result_state, next_context_json, started_at, completed_at, created_at
            FROM jobs WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| Ok(self.row_to_job(row)));
        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_by_kyco_job_id(&self, kyco_job_id: u64) -> Result<Option<BugBountyJob>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, kyco_job_id, mode, target_files_json, prompt,
                   status, result_state, next_context_json, started_at, completed_at, created_at
            FROM jobs
            WHERE kyco_job_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )?;

        let result = stmt.query_row(params![kyco_job_id as i64], |row| Ok(self.row_to_job(row)));
        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_by_project(&self, project_id: &str) -> Result<Vec<BugBountyJob>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, kyco_job_id, mode, target_files_json, prompt,
                   status, result_state, next_context_json, started_at, completed_at, created_at
            FROM jobs
            WHERE project_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let jobs = stmt
            .query_map(params![project_id], |row| Ok(self.row_to_job(row)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(jobs)
    }

    pub fn list_recent_by_project(&self, project_id: &str, limit: usize) -> Result<Vec<BugBountyJob>> {
        let conn = self.db.conn();
        let limit: i64 = limit.try_into().unwrap_or(0);
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, kyco_job_id, mode, target_files_json, prompt,
                   status, result_state, next_context_json, started_at, completed_at, created_at
            FROM jobs
            WHERE project_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )?;

        let jobs = stmt
            .query_map(params![project_id, limit], |row| Ok(self.row_to_job(row)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(jobs)
    }
}

// ============================================
// JOB <-> FINDING LINK REPOSITORY
// ============================================

/// Repository for linking jobs to findings
pub struct JobFindingRepository {
    db: BugBountyDb,
}

impl JobFindingRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    pub fn link(&self, job_id: &str, finding_id: &str, link_type: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            r#"
            INSERT INTO job_findings (job_id, finding_id, link_type, created_at)
            VALUES (?1, ?2, ?3, (strftime('%s', 'now') * 1000))
            ON CONFLICT(job_id, finding_id) DO UPDATE SET
                link_type = excluded.link_type
            "#,
            params![job_id, finding_id, link_type],
        )
        .context("Failed to link job to finding")?;
        Ok(())
    }

    pub fn list_findings_for_job(&self, job_id: &str) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT finding_id FROM job_findings WHERE job_id = ?1 ORDER BY created_at",
        )?;

        let ids = stmt
            .query_map(params![job_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    pub fn list_jobs_for_finding(&self, finding_id: &str) -> Result<Vec<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT job_id FROM job_findings WHERE finding_id = ?1 ORDER BY created_at",
        )?;

        let ids = stmt
            .query_map(params![finding_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    pub fn unlink(&self, job_id: &str, finding_id: &str) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "DELETE FROM job_findings WHERE job_id = ?1 AND finding_id = ?2",
            params![job_id, finding_id],
        )
        .context("Failed to unlink job from finding")?;
        Ok(())
    }

    pub fn is_linked(&self, job_id: &str, finding_id: &str) -> Result<bool> {
        let conn = self.db.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM job_findings WHERE job_id = ?1 AND finding_id = ?2",
            params![job_id, finding_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

// ============================================
// MEMORY REPOSITORY
// ============================================

/// Repository for ProjectMemory CRUD operations
pub struct MemoryRepository {
    db: BugBountyDb,
}

impl MemoryRepository {
    pub fn new(db: BugBountyDb) -> Self {
        Self { db }
    }

    /// Create a new memory entry
    pub fn create(&self, mem: &ProjectMemory) -> Result<i64> {
        let conn = self.db.conn();
        let tags_json = if mem.tags.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&mem.tags)?)
        };

        conn.execute(
            r#"
            INSERT INTO project_memory (
                project_id, memory_type, source_kind, title, content,
                file_path, line_start, line_end, symbol,
                from_file, from_line, to_file, to_line,
                confidence, tags_json, source_job_id, created_at, is_active
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18
            )
            "#,
            params![
                mem.project_id,
                mem.memory_type.as_str(),
                mem.source_kind.as_str(),
                mem.title,
                mem.content,
                mem.file_path,
                mem.line_start,
                mem.line_end,
                mem.symbol,
                mem.from_location.as_ref().map(|l| &l.file),
                mem.from_location.as_ref().and_then(|l| l.line),
                mem.to_location.as_ref().map(|l| &l.file),
                mem.to_location.as_ref().and_then(|l| l.line),
                mem.confidence.map(|c| c.as_str()),
                tags_json,
                mem.source_job_id,
                mem.created_at,
                mem.is_active as i32,
            ],
        )
        .context("Failed to create memory entry")?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    /// Get a memory entry by ID
    pub fn get(&self, id: i64) -> Result<Option<ProjectMemory>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, memory_type, source_kind, title, content,
                   file_path, line_start, line_end, symbol,
                   from_file, from_line, to_file, to_line,
                   confidence, tags_json, source_job_id, created_at, is_active
            FROM project_memory WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| Ok(self.row_to_memory(row)));
        match result {
            Ok(mem) => Ok(Some(mem)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all memory entries for a project
    pub fn list_by_project(&self, project_id: &str) -> Result<Vec<ProjectMemory>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, memory_type, source_kind, title, content,
                   file_path, line_start, line_end, symbol,
                   from_file, from_line, to_file, to_line,
                   confidence, tags_json, source_job_id, created_at, is_active
            FROM project_memory
            WHERE project_id = ?1 AND is_active = 1
            ORDER BY created_at DESC
            "#,
        )?;

        let entries = stmt
            .query_map(params![project_id], |row| Ok(self.row_to_memory(row)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// List memory entries by type
    pub fn list_by_type(&self, project_id: &str, memory_type: MemoryType) -> Result<Vec<ProjectMemory>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, memory_type, source_kind, title, content,
                   file_path, line_start, line_end, symbol,
                   from_file, from_line, to_file, to_line,
                   confidence, tags_json, source_job_id, created_at, is_active
            FROM project_memory
            WHERE project_id = ?1 AND memory_type = ?2 AND is_active = 1
            ORDER BY created_at DESC
            "#,
        )?;

        let entries = stmt
            .query_map(params![project_id, memory_type.as_str()], |row| {
                Ok(self.row_to_memory(row))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// List memory entries by source kind
    pub fn list_by_source_kind(
        &self,
        project_id: &str,
        source_kind: MemorySourceKind,
    ) -> Result<Vec<ProjectMemory>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, memory_type, source_kind, title, content,
                   file_path, line_start, line_end, symbol,
                   from_file, from_line, to_file, to_line,
                   confidence, tags_json, source_job_id, created_at, is_active
            FROM project_memory
            WHERE project_id = ?1 AND source_kind = ?2 AND is_active = 1
            ORDER BY created_at DESC
            "#,
        )?;

        let entries = stmt
            .query_map(params![project_id, source_kind.as_str()], |row| {
                Ok(self.row_to_memory(row))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Check if a duplicate memory entry exists
    pub fn exists_duplicate(&self, mem: &ProjectMemory) -> Result<bool> {
        let conn = self.db.conn();

        // For dataflow, check from/to locations
        if mem.memory_type == MemoryType::Dataflow {
            let from_file = mem.from_location.as_ref().map(|l| &l.file);
            let from_line = mem.from_location.as_ref().and_then(|l| l.line);
            let to_file = mem.to_location.as_ref().map(|l| &l.file);
            let to_line = mem.to_location.as_ref().and_then(|l| l.line);

            let count: i64 = conn.query_row(
                r#"
                SELECT COUNT(*) FROM project_memory
                WHERE project_id = ?1
                  AND memory_type = ?2
                  AND from_file = ?3
                  AND from_line = ?4
                  AND to_file = ?5
                  AND to_line = ?6
                  AND is_active = 1
                "#,
                params![
                    mem.project_id,
                    mem.memory_type.as_str(),
                    from_file,
                    from_line,
                    to_file,
                    to_line,
                ],
                |row| row.get(0),
            )?;
            return Ok(count > 0);
        }

        // For source/sink/note, check file + line
        if mem.file_path.is_some() && mem.line_start.is_some() {
            let count: i64 = conn.query_row(
                r#"
                SELECT COUNT(*) FROM project_memory
                WHERE project_id = ?1
                  AND memory_type = ?2
                  AND file_path = ?3
                  AND line_start = ?4
                  AND is_active = 1
                "#,
                params![
                    mem.project_id,
                    mem.memory_type.as_str(),
                    mem.file_path,
                    mem.line_start,
                ],
                |row| row.get(0),
            )?;
            return Ok(count > 0);
        }

        // For entries without file/line, check by title (notes, context)
        if mem.file_path.is_none() {
            let count: i64 = conn.query_row(
                r#"
                SELECT COUNT(*) FROM project_memory
                WHERE project_id = ?1
                  AND memory_type = ?2
                  AND title = ?3
                  AND is_active = 1
                "#,
                params![mem.project_id, mem.memory_type.as_str(), mem.title,],
                |row| row.get(0),
            )?;
            return Ok(count > 0);
        }

        Ok(false)
    }

    /// Delete a memory entry
    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM project_memory WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Soft-delete a memory entry (set is_active = 0)
    pub fn deactivate(&self, id: i64) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE project_memory SET is_active = 0 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Clear all memory entries of a specific source kind for a project
    pub fn clear_by_source_kind(
        &self,
        project_id: &str,
        source_kind: MemorySourceKind,
    ) -> Result<usize> {
        let conn = self.db.conn();
        let count = conn.execute(
            "DELETE FROM project_memory WHERE project_id = ?1 AND source_kind = ?2",
            params![project_id, source_kind.as_str()],
        )?;
        Ok(count)
    }

    /// Clear all memory entries for a project
    pub fn clear_all(&self, project_id: &str) -> Result<usize> {
        let conn = self.db.conn();
        let count = conn.execute(
            "DELETE FROM project_memory WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(count)
    }

    // Helper to convert a row to ProjectMemory
    fn row_to_memory(&self, row: &rusqlite::Row) -> ProjectMemory {
        let from_file: Option<String> = row.get(10).ok().flatten();
        let from_line: Option<u32> = row.get(11).ok().flatten();
        let to_file: Option<String> = row.get(12).ok().flatten();
        let to_line: Option<u32> = row.get(13).ok().flatten();

        let from_location = from_file.map(|f| {
            let mut loc = MemoryLocation::new(f);
            if let Some(line) = from_line {
                loc = loc.with_line(line);
            }
            loc
        });

        let to_location = to_file.map(|f| {
            let mut loc = MemoryLocation::new(f);
            if let Some(line) = to_line {
                loc = loc.with_line(line);
            }
            loc
        });

        let tags: Vec<String> = row
            .get::<_, Option<String>>(15)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        ProjectMemory {
            id: row.get(0).ok(),
            project_id: row.get(1).unwrap_or_default(),
            memory_type: row
                .get::<_, String>(2)
                .ok()
                .and_then(|s| MemoryType::from_str(&s))
                .unwrap_or(MemoryType::Note),
            source_kind: row
                .get::<_, String>(3)
                .ok()
                .and_then(|s| MemorySourceKind::from_str(&s))
                .unwrap_or(MemorySourceKind::Manual),
            title: row.get(4).unwrap_or_default(),
            content: row.get(5).ok().flatten(),
            file_path: row.get(6).ok().flatten(),
            line_start: row.get(7).ok().flatten(),
            line_end: row.get(8).ok().flatten(),
            symbol: row.get(9).ok().flatten(),
            from_location,
            to_location,
            confidence: row
                .get::<_, Option<String>>(14)
                .ok()
                .flatten()
                .and_then(|s| MemoryConfidence::from_str(&s)),
            tags,
            source_job_id: row.get(16).ok().flatten(),
            created_at: row.get(17).unwrap_or(0),
            is_active: row.get::<_, i32>(18).unwrap_or(1) == 1,
        }
    }
}
