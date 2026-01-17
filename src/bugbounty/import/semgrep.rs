//! Semgrep JSON output importer
//!
//! Semgrep native JSON format (not SARIF) provides more detailed metadata.
//! For SARIF output, use the sarif module instead.

use super::{map_confidence, map_severity, rule_id_to_title, ImportResult};
use crate::bugbounty::{CodeLocation, Finding, FindingStatus, FlowEdge, FlowKind, Severity};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Result of Semgrep import
pub type SemgrepResult = ImportResult;

/// Semgrep JSON output root
#[derive(Debug, Deserialize)]
pub struct SemgrepOutput {
    pub version: Option<String>,
    pub results: Vec<SemgrepResultItem>,
    pub errors: Option<Vec<SemgrepError>>,
    pub paths: Option<SemgrepPaths>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepResultItem {
    pub check_id: String,
    pub path: String,
    pub start: SemgrepPosition,
    pub end: SemgrepPosition,
    pub extra: SemgrepExtra,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepPosition {
    pub line: u32,
    pub col: u32,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepExtra {
    pub message: String,
    pub severity: String,
    pub metadata: Option<SemgrepMetadata>,
    pub lines: Option<String>,
    #[serde(rename = "dataflow_trace")]
    pub dataflow_trace: Option<SemgrepDataflowTrace>,
    pub fingerprint: Option<String>,
    pub is_ignored: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepMetadata {
    pub category: Option<String>,
    pub confidence: Option<String>,
    pub cwe: Option<SemgrepCwe>,
    pub impact: Option<String>,
    pub likelihood: Option<String>,
    pub owasp: Option<Vec<String>>,
    pub references: Option<Vec<String>>,
    pub subcategory: Option<Vec<String>>,
    pub technology: Option<Vec<String>>,
    pub vulnerability_class: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SemgrepCwe {
    Single(String),
    Multiple(Vec<String>),
}

impl SemgrepCwe {
    pub fn first(&self) -> Option<String> {
        match self {
            SemgrepCwe::Single(s) => Some(s.clone()),
            SemgrepCwe::Multiple(v) => v.first().cloned(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SemgrepDataflowTrace {
    pub taint_source: Option<SemgrepTaintLocation>,
    pub intermediate_vars: Option<Vec<SemgrepTaintLocation>>,
    pub taint_sink: Option<SemgrepTaintLocation>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepTaintLocation {
    pub location: SemgrepLocation,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepLocation {
    pub path: String,
    pub start: SemgrepPosition,
    pub end: SemgrepPosition,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepError {
    pub code: Option<i32>,
    pub level: Option<String>,
    pub message: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepPaths {
    pub scanned: Option<Vec<String>>,
    pub skipped: Option<Vec<SemgrepSkippedPath>>,
}

#[derive(Debug, Deserialize)]
pub struct SemgrepSkippedPath {
    pub path: String,
    pub reason: String,
}

/// Import findings from a Semgrep JSON file
pub fn import_semgrep(path: &Path, project_id: &str, start_number: u32) -> Result<SemgrepResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Semgrep file: {}", path.display()))?;

    import_semgrep_str(&content, project_id, start_number)
}

/// Import findings from a Semgrep JSON string
pub fn import_semgrep_str(
    content: &str,
    project_id: &str,
    start_number: u32,
) -> Result<SemgrepResult> {
    let output: SemgrepOutput =
        serde_json::from_str(content).with_context(|| "Failed to parse Semgrep JSON")?;

    let mut result = ImportResult::new();
    let mut finding_number = start_number;

    // Report any Semgrep errors as warnings
    if let Some(errors) = &output.errors {
        for error in errors {
            if let Some(msg) = &error.message {
                result.add_warning(format!("Semgrep error: {}", msg));
            }
        }
    }

    for item in &output.results {
        // Skip ignored results
        if item.extra.is_ignored.unwrap_or(false) {
            result.skipped += 1;
            continue;
        }

        // Generate finding ID
        let finding_id = Finding::generate_id(project_id, finding_number);
        finding_number += 1;

        // Get title from check_id
        let title = rule_id_to_title(&item.check_id);

        // Get severity
        let severity = map_severity(&item.extra.severity).unwrap_or(Severity::Medium);

        // Get confidence from metadata
        let confidence = item
            .extra
            .metadata
            .as_ref()
            .and_then(|m| m.confidence.as_deref())
            .and_then(|c| map_confidence(Some(c)));

        // Get CWE from metadata
        let cwe = item
            .extra
            .metadata
            .as_ref()
            .and_then(|m| m.cwe.as_ref())
            .and_then(|c| c.first())
            .map(|c| {
                if c.starts_with("CWE-") {
                    c.to_string()
                } else {
                    format!("CWE-{}", c)
                }
            });

        // Build affected asset
        let affected_asset = format!("{}:{}", item.path, item.start.line);

        // Build attack scenario from message
        let attack_scenario = format!(
            "{}\n\nLocation: {}:{}-{}",
            item.extra.message, item.path, item.start.line, item.end.line
        );

        // Build impact from metadata
        let impact = item
            .extra
            .metadata
            .as_ref()
            .and_then(|m| m.impact.as_ref())
            .map(|i| {
                format!(
                    "Impact: {}\nLikelihood: {}",
                    i,
                    item.extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.likelihood.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("unknown")
                )
            });

        // Build finding
        let mut finding = Finding::new(&finding_id, project_id, &title)
            .with_severity(severity)
            .with_status(FindingStatus::Raw)
            .with_affected_asset(&affected_asset)
            .with_attack_scenario(&attack_scenario);

        if let Some(conf) = confidence {
            finding = finding.with_confidence(conf);
        }

        if let Some(cwe_id) = cwe {
            finding = finding.with_cwe(&cwe_id);
        }

        if let Some(impact_text) = impact {
            finding = finding.with_impact(&impact_text);
        }

        // Add OWASP references to preconditions
        if let Some(owasp) = item
            .extra
            .metadata
            .as_ref()
            .and_then(|m| m.owasp.as_ref())
        {
            let preconditions = format!("OWASP: {}", owasp.join(", "));
            finding = finding.with_preconditions(&preconditions);
        }

        result.add_finding(finding);

        // Process dataflow trace for taint tracking
        if let Some(trace) = &item.extra.dataflow_trace {
            let mut locations: Vec<CodeLocation> = Vec::new();

            // Source
            if let Some(source) = &trace.taint_source {
                let mut loc = CodeLocation::new(&source.location.path)
                    .with_line(source.location.start.line)
                    .with_column(source.location.start.col);
                if let Some(content) = &source.content {
                    loc = loc.with_snippet(content);
                }
                locations.push(loc);
            }

            // Intermediate variables
            if let Some(intermediates) = &trace.intermediate_vars {
                for intermediate in intermediates {
                    let mut loc = CodeLocation::new(&intermediate.location.path)
                        .with_line(intermediate.location.start.line)
                        .with_column(intermediate.location.start.col);
                    if let Some(content) = &intermediate.content {
                        loc = loc.with_snippet(content);
                    }
                    locations.push(loc);
                }
            }

            // Sink
            if let Some(sink) = &trace.taint_sink {
                let mut loc = CodeLocation::new(&sink.location.path)
                    .with_line(sink.location.start.line)
                    .with_column(sink.location.start.col);
                if let Some(content) = &sink.content {
                    loc = loc.with_snippet(content);
                }
                locations.push(loc);
            }

            // Create edges between consecutive locations
            for window in locations.windows(2) {
                if let [from, to] = window {
                    let edge = FlowEdge::new(&finding_id, from.clone(), to.clone(), FlowKind::Taint);
                    result.add_flow_edge(edge);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bugbounty::Confidence;

    const SAMPLE_SEMGREP: &str = r#"{
        "version": "1.0.0",
        "results": [{
            "check_id": "go.lang.security.audit.sqli.string-formatted-query",
            "path": "src/db/queries.go",
            "start": { "line": 42, "col": 5, "offset": 1234 },
            "end": { "line": 42, "col": 80, "offset": 1309 },
            "extra": {
                "message": "String formatting used in SQL query. This can lead to SQL injection.",
                "severity": "ERROR",
                "metadata": {
                    "category": "security",
                    "confidence": "HIGH",
                    "cwe": "CWE-89",
                    "impact": "HIGH",
                    "likelihood": "HIGH",
                    "owasp": ["A03:2021 - Injection"],
                    "technology": ["go"],
                    "vulnerability_class": ["SQL Injection"]
                },
                "lines": "db.Query(fmt.Sprintf(\"SELECT * FROM users WHERE id = %s\", userInput))",
                "dataflow_trace": {
                    "taint_source": {
                        "location": {
                            "path": "src/handler.go",
                            "start": { "line": 20, "col": 10 },
                            "end": { "line": 20, "col": 30 }
                        },
                        "content": "r.URL.Query().Get(\"id\")"
                    },
                    "intermediate_vars": [{
                        "location": {
                            "path": "src/handler.go",
                            "start": { "line": 25, "col": 5 },
                            "end": { "line": 25, "col": 20 }
                        },
                        "content": "userInput := id"
                    }],
                    "taint_sink": {
                        "location": {
                            "path": "src/db/queries.go",
                            "start": { "line": 42, "col": 5 },
                            "end": { "line": 42, "col": 80 }
                        },
                        "content": "db.Query(fmt.Sprintf(...))"
                    }
                }
            }
        }],
        "errors": [],
        "paths": {
            "scanned": ["src/"],
            "skipped": []
        }
    }"#;

    #[test]
    fn test_import_semgrep() {
        let result = import_semgrep_str(SAMPLE_SEMGREP, "test-project", 1).unwrap();

        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.flow_edges.len(), 2); // source->intermediate, intermediate->sink

        let finding = &result.findings[0];
        assert_eq!(finding.project_id, "test-project");
        assert!(finding.title.contains("String Formatted Query"));
        assert_eq!(finding.severity, Some(Severity::High));
        assert_eq!(finding.confidence, Some(Confidence::High));
        assert_eq!(finding.cwe_id, Some("CWE-89".to_string()));
        assert!(finding.attack_scenario.as_ref().unwrap().contains("SQL"));
    }
}
