//! Parser for structured agent output (`next_context`)
//!
//! Agents can output findings, flow edges, and artifacts in a structured format.
//! This module parses that output and converts it to our domain models.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    Artifact, ArtifactType, Confidence, Finding, FlowEdge, MemoryConfidence, MemoryLocation,
    MemorySourceKind, MemoryType, ProjectMemory, Reachability, Severity,
};

/// Structured output from an agent job
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NextContext {
    /// Findings discovered or updated by this job
    #[serde(default)]
    pub findings: Vec<FindingOutput>,

    /// Flow edges (taint/dataflow paths)
    #[serde(default)]
    pub flow_edges: Vec<FlowEdgeOutput>,

    /// Artifacts (evidence, screenshots, logs)
    #[serde(default)]
    pub artifacts: Vec<ArtifactOutput>,

    /// Memory entries (sources, sinks, dataflows, notes)
    #[serde(default)]
    pub memory: Vec<MemoryOutput>,

    /// State summary for chaining
    #[serde(default)]
    pub state: Option<String>,

    /// Summary for human consumption
    #[serde(default)]
    pub summary: Option<String>,
}

/// Finding as output by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingOutput {
    /// Optional ID (if updating existing finding)
    pub id: Option<String>,

    /// Title (required)
    pub title: String,

    /// Severity level
    pub severity: Option<String>,

    /// Attack scenario - how an attacker exploits this
    pub attack_scenario: Option<String>,

    /// Preconditions - what must be true for exploitation
    pub preconditions: Option<String>,

    /// Reachability - public/auth_required/internal
    pub reachability: Option<String>,

    /// Impact description
    pub impact: Option<String>,

    /// Confidence level
    pub confidence: Option<String>,

    /// CWE ID (e.g., "CWE-639")
    pub cwe_id: Option<String>,

    /// Affected assets/endpoints
    #[serde(default)]
    pub affected_assets: Vec<String>,

    /// Taint path summary
    pub taint_path: Option<String>,
}

/// Flow edge as output by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdgeOutput {
    /// Optional: explicitly associate this edge with a finding
    pub finding_id: Option<String>,

    /// Source location
    pub from_file: Option<String>,
    pub from_line: Option<u32>,
    pub from_symbol: Option<String>,

    /// Destination location
    pub to_file: Option<String>,
    pub to_line: Option<u32>,
    pub to_symbol: Option<String>,

    /// Edge type: taint, authz, dataflow, controlflow
    pub kind: Option<String>,

    /// Notes about this edge
    pub notes: Option<String>,
}

/// Artifact as output by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactOutput {
    /// Optional: explicitly associate this artifact with a finding
    pub finding_id: Option<String>,

    /// Artifact type
    #[serde(rename = "type")]
    pub artifact_type: Option<String>,

    /// Path to the artifact (relative to project root)
    pub path: String,

    /// Optional description
    pub description: Option<String>,

    /// Optional content hash (for deduplication)
    pub hash: Option<String>,
}

/// Memory entry as output by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOutput {
    /// Memory type: source, sink, dataflow, note, context
    #[serde(rename = "type")]
    pub memory_type: String,

    /// Short title/description
    pub title: String,

    /// Optional detailed content
    pub content: Option<String>,

    /// File path (for source, sink, note)
    pub file: Option<String>,

    /// Line number
    pub line: Option<u32>,

    /// Symbol name (function, variable, etc.)
    pub symbol: Option<String>,

    /// Confidence level: high, medium, low
    pub confidence: Option<String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    // Dataflow specific fields
    /// Source file for dataflow
    pub from_file: Option<String>,

    /// Source line for dataflow
    pub from_line: Option<u32>,

    /// Destination file for dataflow
    pub to_file: Option<String>,

    /// Destination line for dataflow
    pub to_line: Option<u32>,
}

impl MemoryOutput {
    /// Convert to domain ProjectMemory model
    pub fn to_memory(&self, project_id: &str, job_id: Option<&str>) -> ProjectMemory {
        let memory_type = MemoryType::from_str(&self.memory_type).unwrap_or(MemoryType::Note);

        let mut mem = ProjectMemory::new(
            project_id,
            memory_type,
            MemorySourceKind::Agent,
            &self.title,
        );

        if let Some(ref content) = self.content {
            mem = mem.with_content(content);
        }

        if let Some(ref file) = self.file {
            mem = mem.with_file(file);
        }

        if let Some(line) = self.line {
            mem = mem.with_line(line);
        }

        if let Some(ref symbol) = self.symbol {
            mem = mem.with_symbol(symbol);
        }

        if let Some(ref conf) = self.confidence {
            if let Some(c) = MemoryConfidence::from_str(conf) {
                mem = mem.with_confidence(c);
            }
        }

        if !self.tags.is_empty() {
            mem = mem.with_tags(self.tags.clone());
        }

        // Dataflow locations
        if let Some(ref from_file) = self.from_file {
            let mut loc = MemoryLocation::new(from_file);
            if let Some(line) = self.from_line {
                loc = loc.with_line(line);
            }
            mem = mem.with_from_location(loc);
        }

        if let Some(ref to_file) = self.to_file {
            let mut loc = MemoryLocation::new(to_file);
            if let Some(line) = self.to_line {
                loc = loc.with_line(line);
            }
            mem = mem.with_to_location(loc);
        }

        if let Some(job_id) = job_id {
            mem = mem.with_source_job(job_id);
        }

        mem
    }
}

impl NextContext {
    /// Parse next_context from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse next_context JSON")
    }

    /// Parse next_context from a JSON value (e.g., `JobResult.next_context`)
    pub fn from_value(value: serde_json::Value) -> Result<Self> {
        serde_json::from_value(value).context("Failed to parse next_context JSON value")
    }

    /// Parse next_context from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse next_context YAML")
    }

    /// Try to extract next_context from agent output text
    ///
    /// Looks for JSON or YAML blocks containing next_context data
    pub fn extract_from_text(text: &str) -> Option<Self> {
        // Try to find JSON block
        if let Some(json) = Self::extract_json_block(text) {
            if let Ok(ctx) = Self::from_json(&json) {
                return Some(ctx);
            }
        }

        // Try to find YAML block
        if let Some(yaml) = Self::extract_yaml_block(text) {
            if let Ok(ctx) = Self::from_yaml(&yaml) {
                return Some(ctx);
            }
        }

        // Try to parse the entire text as JSON
        if let Ok(ctx) = Self::from_json(text) {
            return Some(ctx);
        }

        // Try to parse the entire text as YAML
        if let Ok(ctx) = Self::from_yaml(text) {
            return Some(ctx);
        }

        None
    }

    fn extract_json_block(text: &str) -> Option<String> {
        // Look for ```json ... ``` block
        let start_markers = ["```json", "```JSON"];
        for marker in start_markers {
            if let Some(start) = text.find(marker) {
                let content_start = start + marker.len();
                if let Some(end) = text[content_start..].find("```") {
                    return Some(text[content_start..content_start + end].trim().to_string());
                }
            }
        }

        // Look for { ... } at top level that looks like next_context
        if let Some(start) = text.find('{') {
            // Find matching closing brace
            let mut depth = 0;
            let mut end = start;
            for (i, c) in text[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 && end > start {
                let json = &text[start..end];
                // Check if it looks like next_context
                if json.contains("findings")
                    || json.contains("flow_edges")
                    || json.contains("artifacts")
                    || json.contains("memory")
                {
                    return Some(json.to_string());
                }
            }
        }

        None
    }

    fn extract_yaml_block(text: &str) -> Option<String> {
        // Look for ```yaml ... ``` block
        let start_markers = ["```yaml", "```YAML", "```yml"];
        for marker in start_markers {
            if let Some(start) = text.find(marker) {
                let content_start = start + marker.len();
                if let Some(end) = text[content_start..].find("```") {
                    return Some(text[content_start..content_start + end].trim().to_string());
                }
            }
        }

        // Look for YAML-like structure starting with "findings:" or "next_context:"
        let yaml_markers = ["findings:", "next_context:", "flow_edges:", "artifacts:", "memory:"];
        for marker in yaml_markers {
            if let Some(start) = text.find(marker) {
                // Take from marker to end of text or next markdown block
                let rest = &text[start..];
                let end = rest.find("```").unwrap_or(rest.len());
                return Some(rest[..end].trim().to_string());
            }
        }

        None
    }

    /// Check if this context has any meaningful content
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
            && self.flow_edges.is_empty()
            && self.artifacts.is_empty()
            && self.memory.is_empty()
    }

    /// Validate the "security-audit" output contract (strict).
    ///
    /// This matches the schema injected by `ContextInjector`:
    /// REQUIRED fields: title, attack_scenario, preconditions, reachability, impact, confidence
    ///
    /// Notes:
    /// - Values may be "UNKNOWN - <reason>" and still pass.
    /// - If there are no findings, this returns Ok(()).
    pub fn validate_security_audit(&self) -> Result<()> {
        if self.findings.is_empty() {
            return Ok(());
        }

        fn is_unknown(v: &str) -> bool {
            v.trim_start()
                .to_ascii_uppercase()
                .starts_with("UNKNOWN")
        }

        let mut errors: Vec<String> = Vec::new();

        for (idx, f) in self.findings.iter().enumerate() {
            let prefix = format!("findings[{}]", idx);

            if f.title.trim().is_empty() {
                errors.push(format!("{prefix}.title is empty"));
            }

            let req_text = [
                ("attack_scenario", f.attack_scenario.as_deref()),
                ("preconditions", f.preconditions.as_deref()),
                ("impact", f.impact.as_deref()),
            ];
            for (name, val) in req_text {
                if val.map(str::trim).unwrap_or("").is_empty() {
                    errors.push(format!("{prefix}.{name} is missing"));
                }
            }

            // Reachability (required, enum-like)
            match f.reachability.as_deref().map(str::trim) {
                Some(v) if !v.is_empty() => {
                    if !is_unknown(v) && Reachability::from_str(v).is_none() {
                        errors.push(format!(
                            "{prefix}.reachability is invalid: {v} (use public|auth_required|internal_only or UNKNOWN - ...)"
                        ));
                    }
                }
                _ => errors.push(format!("{prefix}.reachability is missing")),
            }

            // Confidence (required, enum-like)
            match f.confidence.as_deref().map(str::trim) {
                Some(v) if !v.is_empty() => {
                    if !is_unknown(v) && Confidence::from_str(v).is_none() {
                        errors.push(format!(
                            "{prefix}.confidence is invalid: {v} (use high|medium|low or UNKNOWN - ...)"
                        ));
                    }
                }
                _ => errors.push(format!("{prefix}.confidence is missing")),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors.join("; "))
        }
    }

    /// Convert findings to domain model
    pub fn to_findings(&self, project_id: &str, start_number: u32) -> Vec<Finding> {
        self.findings
            .iter()
            .enumerate()
            .map(|(i, f)| f.to_finding(project_id, start_number + i as u32))
            .collect()
    }

    /// Convert flow edges to domain model
    pub fn to_flow_edges(&self, default_finding_id: &str) -> Vec<FlowEdge> {
        self.flow_edges
            .iter()
            .map(|e| e.to_flow_edge(default_finding_id))
            .collect()
    }

    /// Convert artifacts to domain model
    pub fn to_artifacts(&self, finding_id: Option<&str>, job_id: Option<&str>) -> Vec<Artifact> {
        self.artifacts
            .iter()
            .map(|a| a.to_artifact(finding_id, job_id))
            .collect()
    }

    /// Convert memory entries to domain model
    pub fn to_memory(&self, project_id: &str, job_id: Option<&str>) -> Vec<ProjectMemory> {
        self.memory
            .iter()
            .map(|m| m.to_memory(project_id, job_id))
            .collect()
    }
}

impl FindingOutput {
    /// Convert to domain Finding model
    pub fn to_finding(&self, project_id: &str, number: u32) -> Finding {
        let id = self
            .id
            .clone()
            .unwrap_or_else(|| Finding::generate_id(project_id, number));

        let mut finding = Finding::new(&id, project_id, &self.title);

        if let Some(ref s) = self.severity {
            if let Some(sev) = Severity::from_str(s) {
                finding = finding.with_severity(sev);
            }
        }

        if let Some(ref s) = self.attack_scenario {
            finding = finding.with_attack_scenario(s);
        }

        if let Some(ref s) = self.preconditions {
            finding.preconditions = Some(s.clone());
        }

        if let Some(ref s) = self.reachability {
            if let Some(reach) = Reachability::from_str(s) {
                finding = finding.with_reachability(reach);
            }
        }

        if let Some(ref s) = self.impact {
            finding = finding.with_impact(s);
        }

        if let Some(ref s) = self.confidence {
            if let Some(conf) = Confidence::from_str(s) {
                finding = finding.with_confidence(conf);
            }
        }

        if let Some(ref s) = self.cwe_id {
            finding = finding.with_cwe(s);
        }

        for asset in &self.affected_assets {
            finding = finding.with_affected_asset(asset.clone());
        }

        if let Some(ref s) = self.taint_path {
            finding = finding.with_taint_path(s);
        }

        finding
    }
}

impl FlowEdgeOutput {
    /// Convert to domain FlowEdge model
    pub fn to_flow_edge(&self, default_finding_id: &str) -> FlowEdge {
        use super::{CodeLocation, FlowKind};

        let finding_id = self
            .finding_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default_finding_id);

        // Build source location
        let from = {
            let mut loc = CodeLocation::new(self.from_file.as_deref().unwrap_or("unknown"));
            if let Some(line) = self.from_line {
                loc = loc.with_line(line);
            }
            if let Some(ref symbol) = self.from_symbol {
                loc = loc.with_symbol(symbol);
            }
            loc
        };

        // Build destination location
        let to = {
            let mut loc = CodeLocation::new(self.to_file.as_deref().unwrap_or("unknown"));
            if let Some(line) = self.to_line {
                loc = loc.with_line(line);
            }
            if let Some(ref symbol) = self.to_symbol {
                loc = loc.with_symbol(symbol);
            }
            loc
        };

        // Parse kind
        let kind = self
            .kind
            .as_ref()
            .and_then(|k| FlowKind::from_str(k))
            .unwrap_or(FlowKind::Dataflow);

        let mut edge = FlowEdge::new(finding_id, from, to, kind);
        if let Some(ref notes) = self.notes {
            edge = edge.with_notes(notes);
        }
        edge
    }
}

impl ArtifactOutput {
    /// Convert to domain Artifact model
    pub fn to_artifact(&self, finding_id: Option<&str>, job_id: Option<&str>) -> Artifact {
        let artifact_type = self
            .artifact_type
            .as_ref()
            .and_then(|s| ArtifactType::from_str(s))
            .unwrap_or(ArtifactType::Log);

        let mut artifact = Artifact::new(&self.path, artifact_type);
        let fid = self
            .finding_id
            .as_deref()
            .or(finding_id)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(fid) = fid {
            artifact = artifact.with_finding(fid);
        }
        if let Some(jid) = job_id {
            artifact = artifact.with_job(jid);
        }
        if let Some(hash) = self.hash.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            artifact = artifact.with_hash(hash);
        }
        artifact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let json = r#"{
            "findings": [
                {
                    "title": "SQL Injection in login",
                    "severity": "high",
                    "attack_scenario": "Attacker injects SQL via username field",
                    "confidence": "high",
                    "affected_assets": ["/api/login"]
                }
            ],
            "state": "found_vulnerability"
        }"#;

        let ctx = NextContext::from_json(json).unwrap();
        assert_eq!(ctx.findings.len(), 1);
        assert_eq!(ctx.findings[0].title, "SQL Injection in login");
        assert_eq!(ctx.state, Some("found_vulnerability".to_string()));
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
findings:
  - title: IDOR in user endpoint
    severity: high
    attack_scenario: Change user_id to access other users
    affected_assets:
      - /api/users/{id}
state: needs_verification
"#;

        let ctx = NextContext::from_yaml(yaml).unwrap();
        assert_eq!(ctx.findings.len(), 1);
        assert_eq!(ctx.findings[0].title, "IDOR in user endpoint");
    }

    #[test]
    fn test_extract_from_text() {
        let text = r#"
I found a vulnerability in the login endpoint.

```json
{
    "findings": [
        {
            "title": "Hardcoded API Key",
            "severity": "medium",
            "confidence": "high"
        }
    ]
}
```

This should be fixed immediately.
"#;

        let ctx = NextContext::extract_from_text(text).unwrap();
        assert_eq!(ctx.findings.len(), 1);
        assert_eq!(ctx.findings[0].title, "Hardcoded API Key");
    }

    #[test]
    fn test_to_finding() {
        let output = FindingOutput {
            id: None,
            title: "Test Finding".to_string(),
            severity: Some("high".to_string()),
            attack_scenario: Some("Attack description".to_string()),
            preconditions: Some("Must be authenticated".to_string()),
            reachability: Some("auth_required".to_string()),
            impact: Some("Data breach".to_string()),
            confidence: Some("high".to_string()),
            cwe_id: Some("CWE-639".to_string()),
            affected_assets: vec!["/api/test".to_string()],
            taint_path: Some("input -> sink".to_string()),
        };

        let finding = output.to_finding("test-project", 1);
        assert!(finding.id.starts_with("test-project-"));
        assert_eq!(finding.title, "Test Finding");
        assert_eq!(finding.severity, Some(Severity::High));
        assert_eq!(finding.confidence, Some(Confidence::High));
    }
}
