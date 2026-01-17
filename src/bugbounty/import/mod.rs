//! Import module for external tool outputs
//!
//! Supports importing security findings from:
//! - SARIF (Static Analysis Results Interchange Format)
//! - Semgrep JSON output
//! - CodeQL SARIF output
//! - Snyk JSON output
//! - Nuclei JSON/JSONL output

mod nuclei;
mod sarif;
mod semgrep;
mod snyk;

pub use nuclei::{import_nuclei, NucleiResult};
pub use sarif::{import_sarif, SarifResult};
pub use semgrep::{import_semgrep, SemgrepResult};
pub use snyk::{import_snyk, SnykResult};

use crate::bugbounty::{Finding, FlowEdge, Severity, Confidence};

/// Generic import result from any source
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Findings created from the import
    pub findings: Vec<Finding>,
    /// Flow edges created from the import
    pub flow_edges: Vec<FlowEdge>,
    /// Number of results skipped (duplicates, invalid, etc.)
    pub skipped: usize,
    /// Warnings during import
    pub warnings: Vec<String>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
            flow_edges: Vec::new(),
            skipped: 0,
            warnings: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    pub fn add_flow_edge(&mut self, edge: FlowEdge) {
        self.flow_edges.push(edge);
    }

    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn summary(&self) -> String {
        format!(
            "Imported {} findings, {} flow edges ({} skipped, {} warnings)",
            self.findings.len(),
            self.flow_edges.len(),
            self.skipped,
            self.warnings.len()
        )
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Map SARIF/Semgrep severity levels to our severity enum
pub fn map_severity(level: &str) -> Option<Severity> {
    match level.to_lowercase().as_str() {
        "critical" | "crit" => Some(Severity::Critical),
        "error" | "high" => Some(Severity::High),
        "warning" | "medium" => Some(Severity::Medium),
        "note" | "low" => Some(Severity::Low),
        "info" | "informational" | "none" => Some(Severity::Info),
        _ => None,
    }
}

/// Map tool confidence levels to our confidence enum
pub fn map_confidence(confidence: Option<&str>) -> Option<Confidence> {
    match confidence.map(|s| s.to_lowercase()).as_deref() {
        Some("high") | Some("certain") => Some(Confidence::High),
        Some("medium") | Some("firm") => Some(Confidence::Medium),
        Some("low") | Some("tentative") => Some(Confidence::Low),
        _ => None,
    }
}

/// Sanitize a rule ID into a finding title
pub fn rule_id_to_title(rule_id: &str) -> String {
    // Convert rule IDs like "go.lang.security.audit.xss.direct-response-write"
    // to "XSS Direct Response Write"
    rule_id
        .split('.')
        .last()
        .unwrap_or(rule_id)
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            if word.chars().all(|c| c.is_uppercase()) {
                word.to_string()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
