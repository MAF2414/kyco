//! Semgrep JSON memory importer
//!
//! Extracts taint sources, sinks, and dataflow paths from Semgrep output
//! and stores them as project memory entries.

use super::{map_confidence, MemoryImportResult};
use crate::bugbounty::{
    MemoryConfidence, MemoryLocation, MemorySourceKind, ProjectMemory,
};
use anyhow::{Context, Result};
use std::path::Path;

// Reuse Semgrep structures from the main semgrep importer
use super::semgrep::SemgrepOutput;

/// Import memory entries (sources, sinks, dataflows) from a Semgrep JSON file
pub fn import_semgrep_memory(path: &Path, project_id: &str) -> Result<MemoryImportResult> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Semgrep file: {}", path.display()))?;

    import_semgrep_memory_str(&content, project_id)
}

/// Import memory entries from a Semgrep JSON string
pub fn import_semgrep_memory_str(content: &str, project_id: &str) -> Result<MemoryImportResult> {
    let output: SemgrepOutput =
        serde_json::from_str(content).with_context(|| "Failed to parse Semgrep JSON")?;

    let mut result = MemoryImportResult::new();

    // Report any Semgrep errors as warnings
    if let Some(errors) = &output.errors {
        for error in errors {
            if let Some(msg) = &error.message {
                result.add_warning(format!("Semgrep error: {}", msg));
            }
        }
    }

    // Track seen sources/sinks for deduplication
    let mut seen_sources: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
    let mut seen_sinks: std::collections::HashSet<(String, u32)> = std::collections::HashSet::new();
    let mut seen_dataflows: std::collections::HashSet<(String, u32, String, u32)> =
        std::collections::HashSet::new();

    for item in &output.results {
        // Skip ignored results
        if item.extra.is_ignored.unwrap_or(false) {
            result.skipped += 1;
            continue;
        }

        // Process dataflow trace if present
        if let Some(trace) = &item.extra.dataflow_trace {
            // Extract source
            if let Some(source) = &trace.taint_source {
                let key = (
                    source.location.path.clone(),
                    source.location.start.line,
                );
                if !seen_sources.contains(&key) {
                    seen_sources.insert(key);

                    let title = source
                        .content
                        .as_ref()
                        .map(|c| truncate_content(c, 80))
                        .unwrap_or_else(|| "Taint source".to_string());

                    let confidence = item
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.confidence.as_deref())
                        .and_then(|c| map_confidence(Some(c)))
                        .map(|c| match c {
                            crate::bugbounty::Confidence::High => MemoryConfidence::High,
                            crate::bugbounty::Confidence::Medium => MemoryConfidence::Medium,
                            crate::bugbounty::Confidence::Low => MemoryConfidence::Low,
                        });

                    let mut mem = ProjectMemory::source(project_id, MemorySourceKind::Semgrep, title)
                        .with_file(&source.location.path)
                        .with_line(source.location.start.line);

                    if let Some(content) = &source.content {
                        mem = mem.with_content(content);
                    }

                    if let Some(conf) = confidence {
                        mem = mem.with_confidence(conf);
                    }

                    // Add category as tag if available
                    if let Some(category) = item
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.category.as_ref())
                    {
                        mem = mem.with_tag(category);
                    }

                    result.add_memory(mem);
                }
            }

            // Extract sink
            if let Some(sink) = &trace.taint_sink {
                let key = (sink.location.path.clone(), sink.location.start.line);
                if !seen_sinks.contains(&key) {
                    seen_sinks.insert(key);

                    let title = sink
                        .content
                        .as_ref()
                        .map(|c| truncate_content(c, 80))
                        .unwrap_or_else(|| "Taint sink".to_string());

                    let confidence = item
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.confidence.as_deref())
                        .and_then(|c| map_confidence(Some(c)))
                        .map(|c| match c {
                            crate::bugbounty::Confidence::High => MemoryConfidence::High,
                            crate::bugbounty::Confidence::Medium => MemoryConfidence::Medium,
                            crate::bugbounty::Confidence::Low => MemoryConfidence::Low,
                        });

                    let mut mem = ProjectMemory::sink(project_id, MemorySourceKind::Semgrep, title)
                        .with_file(&sink.location.path)
                        .with_line(sink.location.start.line);

                    if let Some(content) = &sink.content {
                        mem = mem.with_content(content);
                    }

                    if let Some(conf) = confidence {
                        mem = mem.with_confidence(conf);
                    }

                    // Add vulnerability class as tag if available
                    if let Some(vuln_class) = item
                        .extra
                        .metadata
                        .as_ref()
                        .and_then(|m| m.vulnerability_class.as_ref())
                        .and_then(|v| v.first())
                    {
                        mem = mem.with_tag(vuln_class);
                    }

                    result.add_memory(mem);
                }
            }

            // Extract dataflow path (source -> sink)
            if let (Some(source), Some(sink)) = (&trace.taint_source, &trace.taint_sink) {
                let key = (
                    source.location.path.clone(),
                    source.location.start.line,
                    sink.location.path.clone(),
                    sink.location.start.line,
                );
                if !seen_dataflows.contains(&key) {
                    seen_dataflows.insert(key);

                    let title = format!(
                        "{} flows to {}",
                        source
                            .content
                            .as_ref()
                            .map(|c| truncate_content(c, 30))
                            .unwrap_or_else(|| "source".to_string()),
                        sink.content
                            .as_ref()
                            .map(|c| truncate_content(c, 30))
                            .unwrap_or_else(|| "sink".to_string())
                    );

                    let from_loc = MemoryLocation::new(&source.location.path)
                        .with_line(source.location.start.line);
                    let to_loc =
                        MemoryLocation::new(&sink.location.path).with_line(sink.location.start.line);

                    let mut mem =
                        ProjectMemory::dataflow(project_id, MemorySourceKind::Semgrep, title)
                            .with_from_location(from_loc)
                            .with_to_location(to_loc)
                            .with_content(&item.extra.message);

                    // Add rule ID as tag
                    mem = mem.with_tag(&item.check_id);

                    result.add_memory(mem);
                }
            }
        }
    }

    Ok(result)
}

/// Truncate content to max length, adding "..." if truncated
fn truncate_content(s: &str, max_len: usize) -> String {
    let s = s.trim();
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bugbounty::MemoryType;

    const SAMPLE_SEMGREP_TAINT: &str = r#"{
        "version": "1.0.0",
        "results": [{
            "check_id": "go.lang.security.audit.sqli.string-formatted-query",
            "path": "src/db/queries.go",
            "start": { "line": 42, "col": 5, "offset": 1234 },
            "end": { "line": 42, "col": 80, "offset": 1309 },
            "extra": {
                "message": "User input flows to SQL query without sanitization",
                "severity": "ERROR",
                "metadata": {
                    "category": "security",
                    "confidence": "HIGH",
                    "vulnerability_class": ["SQL Injection"]
                },
                "dataflow_trace": {
                    "taint_source": {
                        "location": {
                            "path": "src/handler.go",
                            "start": { "line": 20, "col": 10 },
                            "end": { "line": 20, "col": 30 }
                        },
                        "content": "r.URL.Query().Get(\"id\")"
                    },
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
        "errors": []
    }"#;

    #[test]
    fn test_import_semgrep_memory() {
        let result = import_semgrep_memory_str(SAMPLE_SEMGREP_TAINT, "test-project").unwrap();

        // Should have 1 source, 1 sink, 1 dataflow
        assert_eq!(result.memory.len(), 3);

        let sources: Vec<_> = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Source)
            .collect();
        let sinks: Vec<_> = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Sink)
            .collect();
        let dataflows: Vec<_> = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Dataflow)
            .collect();

        assert_eq!(sources.len(), 1);
        assert_eq!(sinks.len(), 1);
        assert_eq!(dataflows.len(), 1);

        // Check source
        let source = &sources[0];
        assert_eq!(source.file_path, Some("src/handler.go".to_string()));
        assert_eq!(source.line_start, Some(20));
        assert_eq!(source.source_kind, MemorySourceKind::Semgrep);

        // Check sink
        let sink = &sinks[0];
        assert_eq!(sink.file_path, Some("src/db/queries.go".to_string()));
        assert_eq!(sink.line_start, Some(42));

        // Check dataflow
        let df = &dataflows[0];
        assert!(df.from_location.is_some());
        assert!(df.to_location.is_some());
        assert_eq!(
            df.from_location.as_ref().unwrap().file,
            "src/handler.go"
        );
        assert_eq!(df.to_location.as_ref().unwrap().file, "src/db/queries.go");
    }

    #[test]
    fn test_deduplication() {
        // Two results with same source/sink should be deduplicated
        let json = r#"{
            "version": "1.0.0",
            "results": [
                {
                    "check_id": "rule1",
                    "path": "src/a.go",
                    "start": { "line": 1, "col": 1 },
                    "end": { "line": 1, "col": 10 },
                    "extra": {
                        "message": "msg1",
                        "severity": "ERROR",
                        "dataflow_trace": {
                            "taint_source": {
                                "location": { "path": "src/input.go", "start": { "line": 10, "col": 1 }, "end": { "line": 10, "col": 5 } },
                                "content": "userInput"
                            },
                            "taint_sink": {
                                "location": { "path": "src/sink.go", "start": { "line": 20, "col": 1 }, "end": { "line": 20, "col": 5 } },
                                "content": "exec()"
                            }
                        }
                    }
                },
                {
                    "check_id": "rule2",
                    "path": "src/b.go",
                    "start": { "line": 2, "col": 1 },
                    "end": { "line": 2, "col": 10 },
                    "extra": {
                        "message": "msg2",
                        "severity": "ERROR",
                        "dataflow_trace": {
                            "taint_source": {
                                "location": { "path": "src/input.go", "start": { "line": 10, "col": 1 }, "end": { "line": 10, "col": 5 } },
                                "content": "userInput"
                            },
                            "taint_sink": {
                                "location": { "path": "src/sink.go", "start": { "line": 20, "col": 1 }, "end": { "line": 20, "col": 5 } },
                                "content": "exec()"
                            }
                        }
                    }
                }
            ],
            "errors": []
        }"#;

        let result = import_semgrep_memory_str(json, "test").unwrap();

        // Should deduplicate: only 1 source, 1 sink, 1 dataflow
        let sources = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Source)
            .count();
        let sinks = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Sink)
            .count();
        let dataflows = result
            .memory
            .iter()
            .filter(|m| m.memory_type == MemoryType::Dataflow)
            .count();

        assert_eq!(sources, 1);
        assert_eq!(sinks, 1);
        assert_eq!(dataflows, 1);
    }
}
