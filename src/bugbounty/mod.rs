//! BugBounty tracking module for KYCo
//!
//! Provides project management, finding tracking (Kanban-style), and flow tracing
//! for security audits and bug bounty programs.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     BugBountyManager                            │
//! │  - Project CRUD                                                 │
//! │  - Finding CRUD + Status transitions                            │
//! │  - Artifact management                                          │
//! │  - Flow trace storage                                           │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//!                    ~/.kyco/bugbounty.db
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let manager = BugBountyManager::new()?;
//!
//! // Create a project
//! let project = Project::new("hackerone-nextcloud", "BugBounty/programs/hackerone-nextcloud")
//!     .with_platform("hackerone");
//! manager.create_project(&project)?;
//!
//! // Create a finding
//! let finding = Finding::new("VULN-001", "hackerone-nextcloud", "IDOR in /api/users")
//!     .with_severity(Severity::High)
//!     .with_confidence(Confidence::High);
//! manager.create_finding(&finding)?;
//!
//! // Update status (Kanban column change)
//! manager.set_finding_status("VULN-001", FindingStatus::Verified)?;
//!
//! // List findings by status
//! let raw_findings = manager.list_findings_by_status(FindingStatus::Raw)?;
//! ```

mod db;
pub mod context_injector;
pub mod import;
pub mod models;
pub mod notes;
pub mod next_context;
mod repository;
mod scope_parser;

pub use context_injector::{ContextInjector, InjectedContext};
pub use db::BugBountyDb;
pub use import::{ImportResult, MemoryImportResult, import_sarif, import_semgrep, import_semgrep_memory};
pub use models::*;
pub use next_context::NextContext;
pub use repository::*;
pub use scope_parser::{parse_scope_file, parse_scope_markdown};

use anyhow::Result;

/// Central manager for BugBounty tracking
///
/// Provides high-level API for managing projects, findings, artifacts, and flow traces.
#[derive(Clone)]
pub struct BugBountyManager {
    db: BugBountyDb,
}

impl BugBountyManager {
    /// Create a new BugBountyManager with the default database location
    pub fn new() -> Result<Self> {
        let db = BugBountyDb::open_default()?;
        Ok(Self { db })
    }

    /// Create a BugBountyManager with a custom database path
    pub fn with_path(path: &std::path::Path) -> Result<Self> {
        let db = BugBountyDb::open(path)?;
        Ok(Self { db })
    }

    /// Get the project repository for CRUD operations
    pub fn projects(&self) -> ProjectRepository {
        ProjectRepository::new(self.db.clone())
    }

    /// Get the finding repository for CRUD operations
    pub fn findings(&self) -> FindingRepository {
        FindingRepository::new(self.db.clone())
    }

    /// Get the artifact repository for CRUD operations
    pub fn artifacts(&self) -> ArtifactRepository {
        ArtifactRepository::new(self.db.clone())
    }

    /// Get the flow edge repository for CRUD operations
    pub fn flow_edges(&self) -> FlowEdgeRepository {
        FlowEdgeRepository::new(self.db.clone())
    }

    /// Get the job repository for CRUD operations
    pub fn jobs(&self) -> JobRepository {
        JobRepository::new(self.db.clone())
    }

    /// Get the job<->finding link repository
    pub fn job_findings(&self) -> JobFindingRepository {
        JobFindingRepository::new(self.db.clone())
    }

    /// Get the memory repository for project memory CRUD operations
    pub fn memory(&self) -> MemoryRepository {
        MemoryRepository::new(self.db.clone())
    }

    /// Reset all data (for testing)
    pub fn reset_all(&self) -> Result<()> {
        self.db.reset_all()
    }

    // ============================================
    // Convenience methods (delegate to repositories)
    // ============================================

    // Projects
    pub fn create_project(&self, project: &Project) -> Result<()> {
        self.projects().create(project)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        self.projects().get(id)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.projects().list()
    }

    /// Infer the best-matching BugBounty project for a given path.
    ///
    /// Returns the matched project plus the resolved absolute project root path.
    ///
    /// Matching strategy:
    /// - If `project.root_path` is absolute: prefix match against `file_path`
    /// - If `project.root_path` is relative: try `work_dir.join(root_path)` prefix match
    /// - Fallback: match `root_path` component sequence anywhere inside `file_path`
    pub fn infer_project_for_path(
        &self,
        work_dir: &std::path::Path,
        file_path: &std::path::Path,
    ) -> Result<Option<(Project, std::path::PathBuf)>> {
        fn best_effort_canonicalize(path: &std::path::Path) -> std::path::PathBuf {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        }

        fn normal_components(path: &std::path::Path) -> Vec<std::ffi::OsString> {
            path.components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(os) => Some(os.to_os_string()),
                    _ => None,
                })
                .collect()
        }

        let file_abs = if file_path.is_absolute() {
            best_effort_canonicalize(file_path)
        } else {
            best_effort_canonicalize(&work_dir.join(file_path))
        };

        // Map normal component index -> full component index for reconstructing absolute roots
        let mut file_normal_components: Vec<std::ffi::OsString> = Vec::new();
        let mut normal_to_full_index: Vec<usize> = Vec::new();
        for (full_index, comp) in file_abs.components().enumerate() {
            if let std::path::Component::Normal(os) = comp {
                file_normal_components.push(os.to_os_string());
                normal_to_full_index.push(full_index);
            }
        }

        let mut best: Option<(usize, Project, std::path::PathBuf)> = None;

        for project in self.list_projects()? {
            let root_raw = std::path::PathBuf::from(&project.root_path);
            if root_raw.as_os_str().is_empty() {
                continue;
            }

            // 1) Absolute prefix match
            if root_raw.is_absolute() {
                let root_abs = best_effort_canonicalize(&root_raw);
                if file_abs.starts_with(&root_abs) {
                    let score = root_abs.components().count();
                    if best
                        .as_ref()
                        .map_or(true, |(best_score, _, _)| score > *best_score)
                    {
                        best = Some((score, project, root_abs));
                    }
                }
                continue;
            }

            // 2) work_dir.join(root_path) prefix match
            let joined = work_dir.join(&root_raw);
            let joined_abs = best_effort_canonicalize(&joined);
            if file_abs.starts_with(&joined_abs) {
                let score = joined_abs.components().count();
                if best
                    .as_ref()
                    .map_or(true, |(best_score, _, _)| score > *best_score)
                {
                    best = Some((score, project, joined_abs));
                }
                continue;
            }

            // 3) Fallback: match the relative root path components anywhere in the file path
            let root_components = normal_components(&root_raw);
            if root_components.is_empty() || root_components.len() > file_normal_components.len() {
                continue;
            }

            let mut matched_root_abs: Option<std::path::PathBuf> = None;
            for start in 0..=file_normal_components.len().saturating_sub(root_components.len()) {
                if file_normal_components[start..start + root_components.len()] == root_components {
                    // Reconstruct absolute root from file_abs components up to end of match
                    let end_full_index = normal_to_full_index[start + root_components.len() - 1];
                    let mut root_abs = std::path::PathBuf::new();
                    for comp in file_abs.components().take(end_full_index + 1) {
                        root_abs.push(comp.as_os_str());
                    }
                    matched_root_abs = Some(root_abs);
                    break;
                }
            }

            let Some(root_abs) = matched_root_abs else { continue };
            let score = root_components.len();
            if best
                .as_ref()
                .map_or(true, |(best_score, _, _)| score > *best_score)
            {
                best = Some((score, project, root_abs));
            }
        }

        Ok(best.map(|(_, p, root)| (p, root)))
    }

    // Findings
    pub fn create_finding(&self, finding: &Finding) -> Result<()> {
        self.findings().create(finding)
    }

    pub fn get_finding(&self, id: &str) -> Result<Option<Finding>> {
        self.findings().get(id)
    }

    pub fn set_finding_status(&self, id: &str, status: FindingStatus) -> Result<()> {
        self.findings().set_status(id, status)
    }

    pub fn list_findings_by_project(&self, project_id: &str) -> Result<Vec<Finding>> {
        self.findings().list_by_project(project_id)
    }

    pub fn list_findings_by_status(&self, status: FindingStatus) -> Result<Vec<Finding>> {
        self.findings().list_by_status(status)
    }

    /// Get the next available finding number for a project
    pub fn next_finding_number(&self, project_id: &str) -> Result<u32> {
        self.findings().next_number(project_id)
    }

    // Artifacts
    pub fn create_artifact(&self, artifact: &Artifact) -> Result<()> {
        self.artifacts().create(artifact)
    }

    pub fn list_artifacts_by_finding(&self, finding_id: &str) -> Result<Vec<Artifact>> {
        self.artifacts().list_by_finding(finding_id)
    }

    // Flow edges
    pub fn create_flow_edge(&self, edge: &FlowEdge) -> Result<()> {
        self.flow_edges().create(edge)
    }

    pub fn get_flow_trace(&self, finding_id: &str) -> Result<FlowTrace> {
        self.flow_edges().get_trace(finding_id)
    }

    // ============================================
    // NextContext processing
    // ============================================

    /// Process agent output and save findings, flow edges, and artifacts
    ///
    /// Returns the IDs of created findings
    pub fn process_next_context(
        &self,
        project_id: &str,
        ctx: &NextContext,
        job_id: Option<&str>,
    ) -> Result<Vec<String>> {
        fn merge_findings(mut existing: Finding, incoming: Finding) -> Finding {
            // Always trust these fields from the incoming structured output
            existing.project_id = incoming.project_id;
            existing.title = incoming.title;

            // Preserve status + FP metadata unless explicitly updated elsewhere.
            // (Structured agent output doesn't carry status transitions today.)

            if incoming.severity.is_some() {
                existing.severity = incoming.severity;
            }
            if incoming.attack_scenario.is_some() {
                existing.attack_scenario = incoming.attack_scenario;
            }
            if incoming.preconditions.is_some() {
                existing.preconditions = incoming.preconditions;
            }
            if incoming.reachability.is_some() {
                existing.reachability = incoming.reachability;
            }
            if incoming.impact.is_some() {
                existing.impact = incoming.impact;
            }
            if incoming.confidence.is_some() {
                existing.confidence = incoming.confidence;
            }
            if incoming.cwe_id.is_some() {
                existing.cwe_id = incoming.cwe_id;
            }
            if incoming.cvss_score.is_some() {
                existing.cvss_score = incoming.cvss_score;
            }
            if !incoming.affected_assets.is_empty() {
                existing.affected_assets = incoming.affected_assets;
            }
            if incoming.taint_path.is_some() {
                existing.taint_path = incoming.taint_path;
            }
            if incoming.notes.is_some() {
                existing.notes = incoming.notes;
            }
            if incoming.source_file.is_some() {
                existing.source_file = incoming.source_file;
            }

            existing
        }

        if let Some(job_id) = job_id {
            // Ensure the job row exists so artifacts and job_findings foreign keys are valid.
            self.jobs().ensure_exists(job_id, Some(project_id))?;
        }

        let mut touched_finding_ids: Vec<String> = Vec::new();

        // Get starting number for new finding IDs (only used when the agent did not provide an ID)
        let start_number = self.next_finding_number(project_id)?;

        // Upsert findings
        let findings = ctx.to_findings(project_id, start_number);
        for finding in findings {
            let finding_id = finding.id.clone();
            match self.get_finding(&finding_id)? {
                Some(existing) => {
                    let merged = merge_findings(existing, finding);
                    self.findings().update(&merged)?;
                }
                None => {
                    self.create_finding(&finding)?;
                }
            }
            touched_finding_ids.push(finding_id);
        }

        let default_finding_id = touched_finding_ids.first().map(|s| s.as_str());

        // Flow edges: support per-edge finding association (`finding_id`), with fallback to
        // the first finding emitted by the agent.
        if !ctx.flow_edges.is_empty() {
            use std::collections::{HashMap, HashSet};

            let touched_set: HashSet<&str> =
                touched_finding_ids.iter().map(|s| s.as_str()).collect();

            let mut edges_by_finding: HashMap<String, Vec<FlowEdge>> = HashMap::new();

            for edge_out in &ctx.flow_edges {
                let fid = edge_out
                    .finding_id
                    .as_deref()
                    .or(default_finding_id)
                    .map(str::trim)
                    .filter(|s| !s.is_empty());

                let Some(fid) = fid else { continue };

                let exists = if touched_set.contains(fid) {
                    true
                } else {
                    self.get_finding(fid)?.is_some()
                };
                if !exists {
                    continue;
                }

                let edge = edge_out.to_flow_edge(fid);
                edges_by_finding
                    .entry(edge.finding_id.clone())
                    .or_default()
                    .push(edge);
            }

            // Replace flow traces per finding to avoid accumulating duplicates.
            for (fid, edges) in edges_by_finding {
                self.flow_edges().delete_for_finding(&fid)?;
                for edge in edges {
                    self.create_flow_edge(&edge)?;
                }
            }
        }

        // Create artifacts (allow per-artifact finding association via `finding_id`).
        let finding_id = default_finding_id;
        let artifacts = ctx.to_artifacts(finding_id, job_id);
        if !artifacts.is_empty() {
            use std::collections::HashSet;
            let touched_set: HashSet<&str> =
                touched_finding_ids.iter().map(|s| s.as_str()).collect();

            for mut artifact in artifacts {
                if let Some(ref fid) = artifact.finding_id {
                    let exists = if touched_set.contains(fid.as_str()) {
                        true
                    } else {
                        self.get_finding(fid)?.is_some()
                    };
                    if !exists {
                        // Keep the artifact, but don't associate it with a missing finding
                        // (avoids FK errors; caller can still query by job_id/path/hash).
                        artifact.finding_id = None;
                    }
                }
                self.create_artifact(&artifact)?;
            }
        }

        // Process memory entries (automatic from agent output)
        if !ctx.memory.is_empty() {
            let memory_entries = ctx.to_memory(project_id, job_id);
            for mem in memory_entries {
                // Deduplication: check if same type + file + line exists
                if !self.memory().exists_duplicate(&mem)? {
                    self.memory().create(&mem)?;
                }
            }
        }

        // Link findings to the job (Kanban provenance)
        if let Some(job_id) = job_id {
            for finding_id in &touched_finding_ids {
                self.job_findings()
                    .link(job_id, finding_id, "discovered")?;
            }
        }

        Ok(touched_finding_ids)
    }

    /// Extract and process next_context from raw agent output text
    pub fn process_agent_output(
        &self,
        project_id: &str,
        output: &str,
        job_id: Option<&str>,
    ) -> Result<Option<Vec<String>>> {
        // Preferred: parse KYCo's structured job result first (supports nested `next_context`).
        if let Some(job_result) = crate::JobResult::parse(output) {
            if let Some(value) = job_result.next_context {
                if let Ok(ctx) = NextContext::from_value(value) {
                    if !ctx.is_empty() {
                        let ids = self.process_next_context(project_id, &ctx, job_id)?;
                        return Ok(Some(ids));
                    }
                }
            }
        }

        // Fallback: accept standalone YAML/JSON blocks in freeform output.
        if let Some(ctx) = NextContext::extract_from_text(output) {
            if !ctx.is_empty() {
                let ids = self.process_next_context(project_id, &ctx, job_id)?;
                return Ok(Some(ids));
            }
        }
        Ok(None)
    }

    // ============================================
    // Import from external tools
    // ============================================

    /// Import findings from a SARIF file
    pub fn import_sarif(&self, path: &std::path::Path, project_id: &str) -> Result<ImportResult> {
        let start_number = self.next_finding_number(project_id)?;
        let result = import::import_sarif(path, project_id, start_number)?;

        // Save findings and flow edges
        for finding in &result.findings {
            self.create_finding(finding)?;
        }
        for edge in &result.flow_edges {
            self.create_flow_edge(edge)?;
        }

        Ok(result)
    }

    /// Import findings from a Semgrep JSON file
    pub fn import_semgrep(&self, path: &std::path::Path, project_id: &str) -> Result<ImportResult> {
        let start_number = self.next_finding_number(project_id)?;
        let result = import::import_semgrep(path, project_id, start_number)?;

        // Save findings and flow edges
        for finding in &result.findings {
            self.create_finding(finding)?;
        }
        for edge in &result.flow_edges {
            self.create_flow_edge(edge)?;
        }

        Ok(result)
    }

    /// Import findings from a Nuclei JSON/JSONL file
    pub fn import_nuclei(&self, path: &std::path::Path, project_id: &str) -> Result<ImportResult> {
        let start_number = self.next_finding_number(project_id)?;
        let result = import::import_nuclei(path, project_id, start_number)?;

        // Save findings (nuclei doesn't have flow edges)
        for finding in &result.findings {
            self.create_finding(finding)?;
        }

        Ok(result)
    }

    /// Import findings from a Snyk JSON file
    pub fn import_snyk(&self, path: &std::path::Path, project_id: &str) -> Result<ImportResult> {
        let start_number = self.next_finding_number(project_id)?;
        let result = import::import_snyk(path, project_id, start_number)?;

        // Save findings (snyk import doesn't currently provide flow edges)
        for finding in &result.findings {
            self.create_finding(finding)?;
        }

        Ok(result)
    }

    /// Auto-detect format and import findings from a file
    pub fn import_auto(&self, path: &std::path::Path, project_id: &str) -> Result<ImportResult> {
        let content = std::fs::read_to_string(path)?;

        // Try to detect format from content
        if content.contains("\"$schema\"") && content.contains("sarif") {
            self.import_sarif(path, project_id)
        } else if content.contains("\"check_id\"") && content.contains("\"extra\"") {
            self.import_semgrep(path, project_id)
        } else if content.contains("\"runs\"") && content.contains("\"results\"") {
            // Generic SARIF without schema
            self.import_sarif(path, project_id)
        } else if content.contains("\"template-id\"") && content.contains("\"matched-at\"") {
            // Nuclei JSONL format
            self.import_nuclei(path, project_id)
        } else if content.lines().next().map(|l| l.contains("\"template-id\"")).unwrap_or(false) {
            // Nuclei JSONL - check first line
            self.import_nuclei(path, project_id)
        } else if content.contains("\"vulnerabilities\"")
            && (content.contains("\"packageName\"") || content.contains("\"packageManager\""))
        {
            // Snyk JSON (deps)
            self.import_snyk(path, project_id)
        } else {
            anyhow::bail!(
                "Could not detect file format. Use --format sarif, semgrep, snyk, or nuclei"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::next_context::{FindingOutput, FlowEdgeOutput};
    use tempfile::tempdir;

    fn test_manager() -> BugBountyManager {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        // Need to keep tempdir alive, so leak it for tests
        std::mem::forget(dir);
        BugBountyManager::with_path(&db_path).unwrap()
    }

    #[test]
    fn test_infer_project_for_path_relative_root() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        let manager = BugBountyManager::with_path(&db_path).unwrap();

        let project_root = dir.path().join("BugBounty/programs/hackerone-nextcloud");
        std::fs::create_dir_all(project_root.join("src")).unwrap();
        let file_path = project_root.join("src/main.rs");
        std::fs::write(&file_path, "fn main() {}\n").unwrap();

        let project = Project::new(
            "hackerone-nextcloud",
            "BugBounty/programs/hackerone-nextcloud",
        )
        .with_platform("hackerone")
        .with_target_name("nextcloud");
        manager.create_project(&project).unwrap();

        let matched = manager
            .infer_project_for_path(dir.path(), &file_path)
            .unwrap();
        assert!(matched.is_some());
        let (matched_project, matched_root) = matched.unwrap();
        assert_eq!(matched_project.id, "hackerone-nextcloud");
        assert!(file_path.canonicalize().unwrap().starts_with(&matched_root));
    }

    #[test]
    fn test_process_next_context_upsert_preserves_status() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        let manager = BugBountyManager::with_path(&db_path).unwrap();

        let project = Project::new("test-project", dir.path().to_string_lossy().to_string());
        manager.create_project(&project).unwrap();

        let existing_id = "test-project-VULN-001";
        let existing = Finding::new(existing_id, "test-project", "Old title")
            .with_status(FindingStatus::Verified)
            .with_severity(Severity::Low);
        manager.create_finding(&existing).unwrap();

        // Old trace should be replaced when new flow_edges are provided.
        manager
            .create_flow_edge(&FlowEdge::new(
                existing_id,
                CodeLocation::new("old.rs").with_line(1),
                CodeLocation::new("old.rs").with_line(2),
                FlowKind::Dataflow,
            ))
            .unwrap();

        let ctx = NextContext {
            findings: vec![FindingOutput {
                id: Some(existing_id.to_string()),
                title: "Updated title".to_string(),
                severity: Some("high".to_string()),
                attack_scenario: Some("Attack".to_string()),
                preconditions: Some("Preconditions".to_string()),
                reachability: Some("public".to_string()),
                impact: Some("Impact".to_string()),
                confidence: Some("high".to_string()),
                cwe_id: Some("CWE-123".to_string()),
                affected_assets: vec!["src/main.rs:1".to_string()],
                taint_path: Some("a -> b".to_string()),
            }],
            flow_edges: vec![FlowEdgeOutput {
                finding_id: None,
                from_file: Some("src/main.rs".to_string()),
                from_line: Some(1),
                from_symbol: Some("main".to_string()),
                to_file: Some("src/db.rs".to_string()),
                to_line: Some(10),
                to_symbol: Some("query".to_string()),
                kind: Some("dataflow".to_string()),
                notes: Some("note".to_string()),
            }],
            artifacts: vec![],
            memory: vec![],
            state: None,
            summary: None,
        };

        let ids = manager
            .process_next_context("test-project", &ctx, Some("job-1"))
            .unwrap();
        assert_eq!(ids, vec![existing_id.to_string()]);

        let updated = manager.get_finding(existing_id).unwrap().unwrap();
        assert_eq!(updated.title, "Updated title");
        assert_eq!(updated.severity, Some(Severity::High));
        assert_eq!(updated.status, FindingStatus::Verified);
        assert_eq!(updated.cwe_id.as_deref(), Some("CWE-123"));

        let trace = manager.get_flow_trace(existing_id).unwrap();
        assert_eq!(trace.edges.len(), 1);
        assert_eq!(trace.edges[0].from.file, "src/main.rs");
        assert_eq!(trace.edges[0].to.file, "src/db.rs");
    }

    #[test]
    fn test_process_next_context_flow_edges_and_artifacts_with_explicit_finding_id() {
        use super::next_context::ArtifactOutput;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_bugbounty.db");
        let manager = BugBountyManager::with_path(&db_path).unwrap();

        let project = Project::new("test-project", dir.path().to_string_lossy().to_string());
        manager.create_project(&project).unwrap();

        let f1 = Finding::new("test-project-VULN-001", "test-project", "First");
        let f2 = Finding::new("test-project-VULN-002", "test-project", "Second");
        manager.create_finding(&f1).unwrap();
        manager.create_finding(&f2).unwrap();

        // Existing flow trace for f1 should not be touched by a ctx that only references f2.
        manager
            .create_flow_edge(&FlowEdge::new(
                &f1.id,
                CodeLocation::new("a.rs").with_line(1),
                CodeLocation::new("b.rs").with_line(2),
                FlowKind::Dataflow,
            ))
            .unwrap();

        let ctx = NextContext {
            findings: vec![],
            flow_edges: vec![FlowEdgeOutput {
                finding_id: Some(f2.id.clone()),
                from_file: Some("src/entry.rs".to_string()),
                from_line: Some(10),
                from_symbol: Some("handler".to_string()),
                to_file: Some("src/sink.rs".to_string()),
                to_line: Some(42),
                to_symbol: Some("db_query".to_string()),
                kind: Some("taint".to_string()),
                notes: Some("explicit finding association".to_string()),
            }],
            artifacts: vec![ArtifactOutput {
                finding_id: Some(f2.id.clone()),
                artifact_type: Some("http_request".to_string()),
                path: "evidence/request.http".to_string(),
                description: Some("PoC request".to_string()),
                hash: Some("deadbeef".to_string()),
            }],
            memory: vec![],
            state: None,
            summary: None,
        };

        let ids = manager
            .process_next_context("test-project", &ctx, None)
            .unwrap();
        assert!(ids.is_empty());

        let trace_f1 = manager.get_flow_trace(&f1.id).unwrap();
        assert_eq!(trace_f1.edges.len(), 1);

        let trace_f2 = manager.get_flow_trace(&f2.id).unwrap();
        assert_eq!(trace_f2.edges.len(), 1);
        assert_eq!(trace_f2.edges[0].from.file, "src/entry.rs");

        let artifacts = manager.list_artifacts_by_finding(&f2.id).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].path, "evidence/request.http");
        assert_eq!(artifacts[0].hash.as_deref(), Some("deadbeef"));

        // Re-ingesting the same context should not duplicate artifacts when hash is present.
        manager
            .process_next_context("test-project", &ctx, None)
            .unwrap();
        let artifacts = manager.list_artifacts_by_finding(&f2.id).unwrap();
        assert_eq!(artifacts.len(), 1);
    }

    #[test]
    fn test_project_workflow() {
        let manager = test_manager();

        // Create project
        let project = Project::new("hackerone-nextcloud", "BugBounty/programs/hackerone-nextcloud")
            .with_platform("hackerone")
            .with_target_name("nextcloud");
        manager.create_project(&project).unwrap();

        // Get project
        let fetched = manager.get_project("hackerone-nextcloud").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.platform, Some("hackerone".to_string()));

        // List projects
        let projects = manager.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
    }

    #[test]
    fn test_finding_workflow() {
        let manager = test_manager();

        // Create project first
        let project = Project::new("test-project", "/path/to/project");
        manager.create_project(&project).unwrap();

        // Create finding
        let finding = Finding::new("VULN-001", "test-project", "Test IDOR")
            .with_severity(Severity::High)
            .with_confidence(Confidence::High);
        manager.create_finding(&finding).unwrap();

        // Get finding
        let fetched = manager.get_finding("VULN-001").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().status, FindingStatus::Raw);

        // Update status
        manager
            .set_finding_status("VULN-001", FindingStatus::Verified)
            .unwrap();

        // Verify status change
        let fetched = manager.get_finding("VULN-001").unwrap().unwrap();
        assert_eq!(fetched.status, FindingStatus::Verified);

        // List by status
        let verified = manager
            .list_findings_by_status(FindingStatus::Verified)
            .unwrap();
        assert_eq!(verified.len(), 1);
    }

    #[test]
    fn test_flow_trace() {
        let manager = test_manager();

        // Create project and finding
        manager
            .create_project(&Project::new("test", "/path"))
            .unwrap();
        manager
            .create_finding(&Finding::new("VULN-001", "test", "SQLi"))
            .unwrap();

        // Create flow edges
        let loc1 = CodeLocation::new("src/handler.rs").with_line(10);
        let loc2 = CodeLocation::new("src/db.rs").with_line(50);
        let edge = FlowEdge::taint("VULN-001", loc1, loc2);
        manager.create_flow_edge(&edge).unwrap();

        // Get trace
        let trace = manager.get_flow_trace("VULN-001").unwrap();
        assert_eq!(trace.edges.len(), 1);
        assert!(trace.summary().contains("handler.rs"));
    }
}
