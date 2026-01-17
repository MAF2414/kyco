//! Nuclei JSON/JSONL output importer
//!
//! Nuclei is a fast vulnerability scanner that outputs findings in JSON format.
//! Each line in the output is a separate JSON object (JSONL format).

use super::{map_severity, ImportResult};
use crate::bugbounty::{Finding, FindingStatus, Severity};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Result of Nuclei import
pub type NucleiResult = ImportResult;

/// Nuclei finding output (per-line JSON)
#[derive(Debug, Deserialize)]
pub struct NucleiFinding {
    /// Template ID (e.g., "cve-2021-44228-log4j")
    #[serde(rename = "template-id")]
    pub template_id: String,

    /// Template name/info
    pub info: Option<NucleiInfo>,

    /// Target host
    pub host: Option<String>,

    /// Matched URL
    #[serde(rename = "matched-at")]
    pub matched_at: Option<String>,

    /// Type of scan (http, dns, network, etc.)
    #[serde(rename = "type")]
    pub scan_type: Option<String>,

    /// IP address
    pub ip: Option<String>,

    /// Timestamp
    pub timestamp: Option<String>,

    /// Curl command to reproduce
    #[serde(rename = "curl-command")]
    pub curl_command: Option<String>,

    /// Matcher name that triggered
    #[serde(rename = "matcher-name")]
    pub matcher_name: Option<String>,

    /// Matcher status
    #[serde(rename = "matcher-status")]
    pub matcher_status: Option<bool>,

    /// Extracted results from matchers
    #[serde(rename = "extracted-results")]
    pub extracted_results: Option<Vec<String>>,

    /// Request that triggered the finding
    pub request: Option<String>,

    /// Response from target
    pub response: Option<String>,

    /// Metadata from template
    pub metadata: Option<serde_json::Value>,
}

/// Nuclei template info block
#[derive(Debug, Deserialize)]
pub struct NucleiInfo {
    /// Template name
    pub name: Option<String>,

    /// Author(s)
    pub author: Option<NucleiAuthor>,

    /// Tags
    pub tags: Option<NucleiTags>,

    /// Severity level
    pub severity: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Reference URLs
    pub reference: Option<NucleiReferences>,

    /// Classification (CVE, CWE, CVSS, etc.)
    pub classification: Option<NucleiClassification>,

    /// Remediation guidance
    pub remediation: Option<String>,
}

/// Author can be string or array
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NucleiAuthor {
    Single(String),
    Multiple(Vec<String>),
}

/// Tags can be string or array
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NucleiTags {
    Single(String),
    Multiple(Vec<String>),
}

impl NucleiTags {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            NucleiTags::Single(s) => s.split(',').map(|t| t.trim().to_string()).collect(),
            NucleiTags::Multiple(v) => v.clone(),
        }
    }
}

/// References can be string or array
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NucleiReferences {
    Single(String),
    Multiple(Vec<String>),
}

impl NucleiReferences {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            NucleiReferences::Single(s) => vec![s.clone()],
            NucleiReferences::Multiple(v) => v.clone(),
        }
    }
}

/// Classification information
#[derive(Debug, Deserialize)]
pub struct NucleiClassification {
    #[serde(rename = "cve-id")]
    pub cve_id: Option<NucleiCveId>,

    #[serde(rename = "cwe-id")]
    pub cwe_id: Option<NucleiCweId>,

    #[serde(rename = "cvss-metrics")]
    pub cvss_metrics: Option<String>,

    #[serde(rename = "cvss-score")]
    pub cvss_score: Option<f64>,
}

/// CVE ID can be string or array
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NucleiCveId {
    Single(String),
    Multiple(Vec<String>),
}

impl NucleiCveId {
    pub fn first(&self) -> Option<String> {
        match self {
            NucleiCveId::Single(s) => Some(s.clone()),
            NucleiCveId::Multiple(v) => v.first().cloned(),
        }
    }
}

/// CWE ID can be string or array
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NucleiCweId {
    Single(String),
    Multiple(Vec<String>),
}

impl NucleiCweId {
    pub fn first(&self) -> Option<String> {
        match self {
            NucleiCweId::Single(s) => Some(s.clone()),
            NucleiCweId::Multiple(v) => v.first().cloned(),
        }
    }
}

/// Map Nuclei severity to our severity
fn nuclei_severity(level: &str) -> Option<Severity> {
    match level.to_lowercase().as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        "medium" => Some(Severity::Medium),
        "low" => Some(Severity::Low),
        "info" | "informational" | "unknown" => Some(Severity::Info),
        _ => map_severity(level),
    }
}

/// Import findings from a Nuclei JSON/JSONL file
pub fn import_nuclei(path: &Path, project_id: &str, start_number: u32) -> Result<NucleiResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Nuclei file: {}", path.display()))?;

    import_nuclei_str(&content, project_id, start_number)
}

/// Import findings from a Nuclei JSON/JSONL string
pub fn import_nuclei_str(content: &str, project_id: &str, start_number: u32) -> Result<NucleiResult> {
    let mut result = ImportResult::new();
    let mut finding_number = start_number;

    // Nuclei outputs JSONL (one JSON per line)
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let finding: NucleiFinding = match serde_json::from_str(line) {
            Ok(f) => f,
            Err(e) => {
                result.add_warning(format!("Line {}: Failed to parse JSON: {}", line_num + 1, e));
                result.skipped += 1;
                continue;
            }
        };

        // Generate finding ID
        let finding_id = Finding::generate_id(project_id, finding_number);
        finding_number += 1;

        // Get title from info.name or template_id
        let title = finding
            .info
            .as_ref()
            .and_then(|i| i.name.clone())
            .unwrap_or_else(|| template_id_to_title(&finding.template_id));

        // Get severity
        let severity = finding
            .info
            .as_ref()
            .and_then(|i| i.severity.as_ref())
            .and_then(|s| nuclei_severity(s))
            .unwrap_or(Severity::Info);

        // Build affected asset from matched_at or host
        let affected_asset = finding
            .matched_at
            .clone()
            .or_else(|| finding.host.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Build attack scenario
        let mut scenario_parts = Vec::new();

        if let Some(ref info) = finding.info {
            if let Some(ref desc) = info.description {
                scenario_parts.push(desc.clone());
            }
        }

        scenario_parts.push(format!("Template: {}", finding.template_id));

        if let Some(ref matched) = finding.matched_at {
            scenario_parts.push(format!("Matched at: {}", matched));
        }

        if let Some(ref scan_type) = finding.scan_type {
            scenario_parts.push(format!("Scan type: {}", scan_type));
        }

        if let Some(ref matcher) = finding.matcher_name {
            scenario_parts.push(format!("Matcher: {}", matcher));
        }

        if let Some(ref extracted) = finding.extracted_results {
            if !extracted.is_empty() {
                scenario_parts.push(format!("Extracted: {}", extracted.join(", ")));
            }
        }

        let attack_scenario = scenario_parts.join("\n\n");

        // Get CWE from classification
        let cwe = finding
            .info
            .as_ref()
            .and_then(|i| i.classification.as_ref())
            .and_then(|c| c.cwe_id.as_ref())
            .and_then(|c| c.first())
            .map(|c| {
                if c.starts_with("CWE-") {
                    c.to_string()
                } else {
                    format!("CWE-{}", c)
                }
            });

        // Build impact from CVE/CVSS if available
        let impact = finding.info.as_ref().and_then(|i| {
            let mut parts = Vec::new();

            if let Some(ref class) = i.classification {
                if let Some(ref cve) = class.cve_id {
                    if let Some(cve_str) = cve.first() {
                        parts.push(format!("CVE: {}", cve_str));
                    }
                }
                if let Some(score) = class.cvss_score {
                    parts.push(format!("CVSS: {:.1}", score));
                }
                if let Some(ref metrics) = class.cvss_metrics {
                    parts.push(format!("CVSS Metrics: {}", metrics));
                }
            }

            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        });

        // Build preconditions from tags
        let preconditions = finding.info.as_ref().and_then(|i| {
            i.tags.as_ref().map(|t| format!("Tags: {}", t.as_vec().join(", ")))
        });

        // Build finding
        let mut f = Finding::new(&finding_id, project_id, &title)
            .with_severity(severity)
            .with_status(FindingStatus::Raw)
            .with_affected_asset(&affected_asset)
            .with_attack_scenario(&attack_scenario);

        if let Some(cwe_id) = cwe {
            f = f.with_cwe(&cwe_id);
        }

        if let Some(impact_text) = impact {
            f = f.with_impact(&impact_text);
        }

        if let Some(precond) = preconditions {
            f.preconditions = Some(precond);
        }

        // Add references to notes if available
        if let Some(ref info) = finding.info {
            if let Some(ref refs) = info.reference {
                let refs_str = refs.as_vec().join("\n");
                if !refs_str.is_empty() {
                    let current = f.attack_scenario.unwrap_or_default();
                    f.attack_scenario = Some(format!("{}\n\nReferences:\n{}", current, refs_str));
                }
            }

            // Add remediation if available
            if let Some(ref remediation) = info.remediation {
                let current = f.attack_scenario.unwrap_or_default();
                f.attack_scenario = Some(format!("{}\n\nRemediation:\n{}", current, remediation));
            }
        }

        // Store curl command as taint_path for easy reproduction
        if let Some(ref curl) = finding.curl_command {
            f.taint_path = Some(format!("Reproduce with:\n{}", curl));
        }

        result.add_finding(f);
    }

    Ok(result)
}

/// Convert template ID to human-readable title
fn template_id_to_title(template_id: &str) -> String {
    // Convert "cve-2021-44228-log4j" to "CVE 2021 44228 Log4j"
    template_id
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            // Keep CVE/CWE uppercase
            if word.eq_ignore_ascii_case("cve") || word.eq_ignore_ascii_case("cwe") {
                word.to_uppercase()
            } else if word.chars().all(|c| c.is_ascii_digit()) {
                word.to_string()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_NUCLEI: &str = r#"{"template-id":"cve-2021-44228-log4j","info":{"name":"Apache Log4j RCE","author":"pdteam","severity":"critical","description":"Apache Log4j2 is vulnerable to RCE via JNDI lookup.","reference":["https://nvd.nist.gov/vuln/detail/CVE-2021-44228"],"classification":{"cve-id":"CVE-2021-44228","cwe-id":"CWE-502","cvss-metrics":"CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H","cvss-score":10.0},"tags":"cve,cve2021,rce,log4j,apache"},"host":"https://target.com","matched-at":"https://target.com/api/login","type":"http","curl-command":"curl -X POST https://target.com/api/login -d '${jndi:ldap://...}'","timestamp":"2024-01-15T10:30:00Z"}
{"template-id":"exposed-panels","info":{"name":"Admin Panel Exposed","severity":"medium","description":"Administrative panel is publicly accessible.","tags":"panel,exposure"},"host":"https://target.com","matched-at":"https://target.com/admin","type":"http"}"#;

    #[test]
    fn test_import_nuclei() {
        let result = import_nuclei_str(SAMPLE_NUCLEI, "test-project", 1).unwrap();

        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.skipped, 0);

        // First finding: Log4j RCE
        let f1 = &result.findings[0];
        assert_eq!(f1.project_id, "test-project");
        assert!(f1.title.contains("Log4j"));
        assert_eq!(f1.severity, Some(Severity::Critical));
        assert_eq!(f1.cwe_id, Some("CWE-502".to_string()));
        assert!(f1.impact.as_ref().unwrap().contains("CVE-2021-44228"));
        assert!(f1.impact.as_ref().unwrap().contains("10.0"));
        assert!(f1.taint_path.as_ref().unwrap().contains("curl"));

        // Second finding: Admin panel
        let f2 = &result.findings[1];
        assert!(f2.title.contains("Admin Panel"));
        assert_eq!(f2.severity, Some(Severity::Medium));
    }

    #[test]
    fn test_template_id_to_title() {
        assert_eq!(template_id_to_title("cve-2021-44228-log4j"), "CVE 2021 44228 Log4j");
        assert_eq!(template_id_to_title("exposed-panels"), "Exposed Panels");
        assert_eq!(template_id_to_title("ssl_tls_version"), "Ssl Tls Version");
    }
}
