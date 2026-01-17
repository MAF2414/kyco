//! SARIF (Static Analysis Results Interchange Format) importer
//!
//! SARIF is the standard format used by many security tools:
//! - Semgrep (with --sarif flag)
//! - CodeQL
//! - Bandit
//! - ESLint (with formatter)
//! - Many others
//!
//! Spec: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html

use super::{map_confidence, map_severity, rule_id_to_title, ImportResult};
use crate::bugbounty::{CodeLocation, Finding, FindingStatus, FlowEdge, FlowKind, Severity};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Result of SARIF import
pub type SarifResult = ImportResult;

/// SARIF root structure
#[derive(Debug, Deserialize)]
pub struct Sarif {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub version: Option<String>,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Option<Vec<SarifResultItem>>,
    #[serde(rename = "originalUriBaseIds")]
    pub original_uri_base_ids: Option<HashMap<String, SarifUri>>,
}

#[derive(Debug, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: Option<String>,
    pub rules: Option<Vec<SarifRule>>,
}

#[derive(Debug, Deserialize)]
pub struct SarifRule {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "shortDescription")]
    pub short_description: Option<SarifMessage>,
    #[serde(rename = "fullDescription")]
    pub full_description: Option<SarifMessage>,
    #[serde(rename = "defaultConfiguration")]
    pub default_configuration: Option<SarifConfiguration>,
    pub properties: Option<SarifRuleProperties>,
}

#[derive(Debug, Deserialize)]
pub struct SarifRuleProperties {
    pub precision: Option<String>,
    pub security_severity: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "cwe")]
    pub cwe: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifConfiguration {
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifMessage {
    pub text: Option<String>,
    pub markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifUri {
    pub uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifResultItem {
    #[serde(rename = "ruleId")]
    pub rule_id: Option<String>,
    #[serde(rename = "ruleIndex")]
    pub rule_index: Option<usize>,
    pub level: Option<String>,
    pub message: SarifMessage,
    pub locations: Option<Vec<SarifLocation>>,
    #[serde(rename = "codeFlows")]
    pub code_flows: Option<Vec<SarifCodeFlow>>,
    pub fingerprints: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: Option<SarifPhysicalLocation>,
}

#[derive(Debug, Deserialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: Option<SarifArtifactLocation>,
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: Option<String>,
    #[serde(rename = "uriBaseId")]
    pub uri_base_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: Option<u32>,
    #[serde(rename = "startColumn")]
    pub start_column: Option<u32>,
    #[serde(rename = "endLine")]
    pub end_line: Option<u32>,
    #[serde(rename = "endColumn")]
    pub end_column: Option<u32>,
    pub snippet: Option<SarifSnippet>,
}

#[derive(Debug, Deserialize)]
pub struct SarifSnippet {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SarifCodeFlow {
    #[serde(rename = "threadFlows")]
    pub thread_flows: Vec<SarifThreadFlow>,
}

#[derive(Debug, Deserialize)]
pub struct SarifThreadFlow {
    pub locations: Vec<SarifThreadFlowLocation>,
}

#[derive(Debug, Deserialize)]
pub struct SarifThreadFlowLocation {
    pub location: SarifLocation,
    #[serde(rename = "nestingLevel")]
    pub nesting_level: Option<u32>,
    pub kinds: Option<Vec<String>>,
}

/// Import findings from a SARIF file
pub fn import_sarif(path: &Path, project_id: &str, start_number: u32) -> Result<SarifResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read SARIF file: {}", path.display()))?;

    import_sarif_str(&content, project_id, start_number)
}

/// Import findings from a SARIF JSON string
pub fn import_sarif_str(content: &str, project_id: &str, start_number: u32) -> Result<SarifResult> {
    let sarif: Sarif =
        serde_json::from_str(content).with_context(|| "Failed to parse SARIF JSON")?;

    let mut result = ImportResult::new();
    let mut finding_number = start_number;

    for run in &sarif.runs {
        // Build rule lookup
        let rules: HashMap<String, &SarifRule> = run
            .tool
            .driver
            .rules
            .as_ref()
            .map(|rules| rules.iter().map(|r| (r.id.clone(), r)).collect())
            .unwrap_or_default();

        let results = match &run.results {
            Some(r) => r,
            None => continue,
        };

        for item in results {
            let rule_id = match &item.rule_id {
                Some(id) => id.clone(),
                None => {
                    result.skipped += 1;
                    result.add_warning("Skipped result without rule_id");
                    continue;
                }
            };

            // Get rule details
            let rule = rules.get(&rule_id);

            // Build finding ID
            let finding_id = Finding::generate_id(project_id, finding_number);
            finding_number += 1;

            // Get title from rule or generate from ID
            let title = rule
                .and_then(|r| r.name.clone())
                .or_else(|| {
                    rule.and_then(|r| r.short_description.as_ref())
                        .and_then(|d| d.text.clone())
                })
                .unwrap_or_else(|| rule_id_to_title(&rule_id));

            // Get severity
            let severity = item
                .level
                .as_deref()
                .or_else(|| {
                    rule.and_then(|r| r.default_configuration.as_ref())
                        .and_then(|c| c.level.as_deref())
                })
                .and_then(map_severity)
                .unwrap_or(Severity::Medium);

            // Get confidence
            let confidence = rule
                .and_then(|r| r.properties.as_ref())
                .and_then(|p| p.precision.as_deref())
                .and_then(|p| map_confidence(Some(p)));

            // Get CWE
            let cwe = rule
                .and_then(|r| r.properties.as_ref())
                .and_then(|p| p.cwe.clone())
                .map(|c| {
                    if c.starts_with("CWE-") {
                        c
                    } else {
                        format!("CWE-{}", c)
                    }
                });

            // Get description
            let description = item.message.text.clone().or_else(|| {
                rule.and_then(|r| r.full_description.as_ref())
                    .and_then(|d| d.text.clone())
            });

            // Get affected assets from locations
            let affected_assets: Vec<String> = item
                .locations
                .as_ref()
                .map(|locs| {
                    locs.iter()
                        .filter_map(|loc| {
                            loc.physical_location.as_ref().and_then(|pl| {
                                pl.artifact_location
                                    .as_ref()
                                    .and_then(|al| al.uri.clone())
                                    .map(|uri| {
                                        if let Some(region) = &pl.region {
                                            if let Some(line) = region.start_line {
                                                return format!("{}:{}", uri, line);
                                            }
                                        }
                                        uri
                                    })
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Build the finding
            let mut finding = Finding::new(&finding_id, project_id, &title)
                .with_severity(severity)
                .with_status(FindingStatus::Raw);

            if let Some(conf) = confidence {
                finding = finding.with_confidence(conf);
            }

            if let Some(cwe_id) = cwe {
                finding = finding.with_cwe(&cwe_id);
            }

            if let Some(desc) = description {
                finding = finding.with_attack_scenario(&desc);
            }

            for asset in affected_assets {
                finding = finding.with_affected_asset(&asset);
            }

            result.add_finding(finding);

            // Process code flows for taint tracking
            if let Some(code_flows) = &item.code_flows {
                for flow in code_flows {
                    for thread_flow in &flow.thread_flows {
                        let locations: Vec<CodeLocation> = thread_flow
                            .locations
                            .iter()
                            .filter_map(|tfl| {
                                tfl.location.physical_location.as_ref().and_then(|pl| {
                                    pl.artifact_location
                                        .as_ref()
                                        .and_then(|al| al.uri.clone())
                                        .map(|uri| {
                                            let mut loc = CodeLocation::new(&uri);
                                            if let Some(region) = &pl.region {
                                                if let Some(line) = region.start_line {
                                                    loc = loc.with_line(line);
                                                }
                                                if let Some(col) = region.start_column {
                                                    loc = loc.with_column(col);
                                                }
                                                if let Some(snippet) = &region.snippet {
                                                    if let Some(text) = &snippet.text {
                                                        loc = loc.with_snippet(text);
                                                    }
                                                }
                                            }
                                            loc
                                        })
                                })
                            })
                            .collect();

                        // Create edges between consecutive locations
                        for window in locations.windows(2) {
                            if let [from, to] = window {
                                let edge =
                                    FlowEdge::new(&finding_id, from.clone(), to.clone(), FlowKind::Taint);
                                result.add_flow_edge(edge);
                            }
                        }
                    }
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

    const SAMPLE_SARIF: &str = r#"{
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "semgrep",
                    "version": "1.0.0",
                    "rules": [{
                        "id": "go.lang.security.audit.xss.direct-response-write",
                        "name": "Direct Response Write XSS",
                        "shortDescription": {
                            "text": "Potential XSS vulnerability"
                        },
                        "defaultConfiguration": {
                            "level": "warning"
                        },
                        "properties": {
                            "precision": "high",
                            "cwe": "79"
                        }
                    }]
                }
            },
            "results": [{
                "ruleId": "go.lang.security.audit.xss.direct-response-write",
                "level": "warning",
                "message": {
                    "text": "User input flows into response without escaping"
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": "src/handler.go"
                        },
                        "region": {
                            "startLine": 42,
                            "startColumn": 5
                        }
                    }
                }],
                "codeFlows": [{
                    "threadFlows": [{
                        "locations": [
                            {
                                "location": {
                                    "physicalLocation": {
                                        "artifactLocation": { "uri": "src/handler.go" },
                                        "region": { "startLine": 30 }
                                    }
                                }
                            },
                            {
                                "location": {
                                    "physicalLocation": {
                                        "artifactLocation": { "uri": "src/handler.go" },
                                        "region": { "startLine": 42 }
                                    }
                                }
                            }
                        ]
                    }]
                }]
            }]
        }]
    }"#;

    #[test]
    fn test_import_sarif() {
        let result = import_sarif_str(SAMPLE_SARIF, "test-project", 1).unwrap();

        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.flow_edges.len(), 1);

        let finding = &result.findings[0];
        assert_eq!(finding.project_id, "test-project");
        assert!(finding.title.contains("XSS") || finding.title.contains("Direct Response Write"));
        assert_eq!(finding.severity, Some(Severity::Medium));
        assert_eq!(finding.confidence, Some(Confidence::High));
        assert_eq!(finding.cwe_id, Some("CWE-79".to_string()));
    }
}
