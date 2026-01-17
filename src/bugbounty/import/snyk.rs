//! Snyk JSON importer
//!
//! Supported inputs (best-effort):
//! - `snyk test --json` (Open Source deps)
//! - `snyk test --all-projects --json` (array of results)
//!
//! Note: If you already have SARIF output (e.g. `--sarif`), prefer the SARIF importer.

use super::ImportResult;
use crate::bugbounty::{Finding, FindingStatus, Severity};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Result of Snyk import
pub type SnykResult = ImportResult;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SnykRoot {
    Single(SnykReport),
    Multiple(Vec<SnykReport>),
}

#[derive(Debug, Deserialize)]
struct SnykReport {
    #[serde(default)]
    vulnerabilities: Vec<SnykVulnerability>,
    #[serde(rename = "projectName")]
    project_name: Option<String>,
    path: Option<String>,
    #[serde(rename = "displayTargetFile")]
    display_target_file: Option<String>,
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SnykVulnerability {
    id: Option<String>,
    title: Option<String>,
    severity: Option<String>,
    #[serde(rename = "packageName")]
    package_name: Option<String>,
    version: Option<String>,
    #[serde(default)]
    from: Vec<String>,
    description: Option<String>,
    identifiers: Option<SnykIdentifiers>,
}

#[derive(Debug, Deserialize)]
struct SnykIdentifiers {
    #[serde(rename = "CWE")]
    cwe: Option<Vec<String>>,
    #[serde(rename = "CVE")]
    cve: Option<Vec<String>>,
}

fn map_snyk_severity(severity: &str) -> Option<Severity> {
    match severity.to_lowercase().as_str() {
        "critical" => Some(Severity::Critical),
        "high" => Some(Severity::High),
        "medium" => Some(Severity::Medium),
        "low" => Some(Severity::Low),
        "info" | "informational" => Some(Severity::Info),
        _ => None,
    }
}

fn normalize_cwe(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.to_uppercase().starts_with("CWE-") {
        trimmed.to_string()
    } else {
        format!("CWE-{}", trimmed)
    }
}

fn report_context_lines(report: &SnykReport) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(ref name) = report.project_name {
        if !name.trim().is_empty() {
            lines.push(format!("- Project: {}", name.trim()));
        }
    }
    if let Some(ref target) = report.display_target_file {
        if !target.trim().is_empty() {
            lines.push(format!("- Target: {}", target.trim()));
        }
    }
    if let Some(ref pm) = report.package_manager {
        if !pm.trim().is_empty() {
            lines.push(format!("- Package manager: {}", pm.trim()));
        }
    }
    if let Some(ref path) = report.path {
        if !path.trim().is_empty() {
            lines.push(format!("- Path: {}", path.trim()));
        }
    }
    lines
}

fn build_attack_scenario(report: &SnykReport, vuln: &SnykVulnerability) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("Imported from Snyk.".to_string());

    if let Some(ref id) = vuln.id {
        if !id.trim().is_empty() {
            lines.push(format!("- Snyk ID: {}", id.trim()));
        }
    }

    if let Some(ref pkg) = vuln.package_name {
        let pkg = pkg.trim();
        if !pkg.is_empty() {
            let ver = vuln.version.as_deref().unwrap_or("").trim();
            if ver.is_empty() {
                lines.push(format!("- Package: {}", pkg));
            } else {
                lines.push(format!("- Package: {}@{}", pkg, ver));
            }
        }
    }

    if !vuln.from.is_empty() {
        let from = vuln
            .from
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" > ");
        if !from.is_empty() {
            lines.push(format!("- Dependency path: {}", from));
        }
    }

    let ctx = report_context_lines(report);
    if !ctx.is_empty() {
        lines.push(String::new());
        lines.push("Context:".to_string());
        lines.extend(ctx);
    }

    if let Some(ref desc) = vuln.description {
        let desc = desc.trim();
        if !desc.is_empty() {
            lines.push(String::new());
            lines.push(desc.to_string());
        }
    }

    lines.join("\n")
}

fn build_affected_assets(vuln: &SnykVulnerability) -> Vec<String> {
    let mut assets = Vec::new();
    if let Some(ref pkg) = vuln.package_name {
        let pkg = pkg.trim();
        if !pkg.is_empty() {
            let ver = vuln.version.as_deref().unwrap_or("").trim();
            if ver.is_empty() {
                assets.push(pkg.to_string());
            } else {
                assets.push(format!("{}@{}", pkg, ver));
            }
        }
    }
    assets
}

/// Import findings from a Snyk JSON file.
pub fn import_snyk(path: &Path, project_id: &str, start_number: u32) -> Result<SnykResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Snyk file: {}", path.display()))?;

    import_snyk_str(&content, project_id, start_number)
}

/// Import findings from a Snyk JSON string.
pub fn import_snyk_str(content: &str, project_id: &str, start_number: u32) -> Result<SnykResult> {
    let parsed: SnykRoot =
        serde_json::from_str(content).with_context(|| "Failed to parse Snyk JSON")?;

    let reports: Vec<SnykReport> = match parsed {
        SnykRoot::Single(r) => vec![r],
        SnykRoot::Multiple(v) => v,
    };

    let mut result = ImportResult::new();
    let mut finding_number = start_number;

    let mut total_vulns = 0usize;
    for report in reports {
        for vuln in &report.vulnerabilities {
            total_vulns += 1;

            let finding_id = Finding::generate_id(project_id, finding_number);
            finding_number += 1;

            let title = vuln
                .title
                .clone()
                .or_else(|| vuln.id.clone())
                .unwrap_or_else(|| "Snyk issue".to_string());

            let severity = vuln
                .severity
                .as_deref()
                .and_then(map_snyk_severity)
                .unwrap_or(Severity::Medium);

            let mut finding = Finding::new(&finding_id, project_id, &title)
                .with_severity(severity)
                .with_status(FindingStatus::Raw);

            if let Some(ref identifiers) = vuln.identifiers {
                if let Some(cwe) = identifiers
                    .cwe
                    .as_ref()
                    .and_then(|v| v.iter().find(|s| !s.trim().is_empty()))
                {
                    finding = finding.with_cwe(normalize_cwe(cwe));
                }
            }

            for asset in build_affected_assets(vuln) {
                finding = finding.with_affected_asset(asset);
            }

            let scenario = build_attack_scenario(&report, vuln);
            if !scenario.trim().is_empty() {
                finding = finding.with_attack_scenario(scenario);
            }

            result.add_finding(finding);
        }
    }

    if total_vulns == 0 {
        result.add_warning("No vulnerabilities found in Snyk output");
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SNYK: &str = r#"{
      "ok": false,
      "path": "/tmp/app",
      "projectName": "app",
      "displayTargetFile": "package.json",
      "packageManager": "npm",
      "vulnerabilities": [
        {
          "id": "SNYK-JS-LODASH-567746",
          "title": "Prototype Pollution",
          "severity": "high",
          "packageName": "lodash",
          "version": "4.17.15",
          "from": ["app@1.0.0", "lodash@4.17.15"],
          "identifiers": { "CWE": ["CWE-1321"], "CVE": ["CVE-2020-8203"] },
          "description": "Some description."
        }
      ]
    }"#;

    #[test]
    fn test_import_snyk_single_report() {
        let result = import_snyk_str(SAMPLE_SNYK, "test-project", 1).unwrap();
        assert_eq!(result.findings.len(), 1);

        let finding = &result.findings[0];
        assert_eq!(finding.id, "test-project-VULN-001");
        assert_eq!(finding.project_id, "test-project");
        assert_eq!(finding.severity, Some(Severity::High));
        assert_eq!(finding.cwe_id, Some("CWE-1321".to_string()));
        assert!(finding
            .affected_assets
            .iter()
            .any(|a| a == "lodash@4.17.15"));
        assert!(finding
            .attack_scenario
            .as_ref()
            .is_some_and(|s| s.contains("SNYK-JS-LODASH-567746")));
    }
}
