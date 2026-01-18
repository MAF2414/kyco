//! Result section rendering for the detail panel

use eframe::egui::{self, RichText};

use crate::gui::theme::{
    ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::Job;

use super::markdown::render_markdown_scroll;

/// Render result section (from YAML block or raw text)
pub(super) fn render_result_section(
    ui: &mut egui::Ui,
    job: &Job,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
) {
    let response_text = job
        .full_response
        .as_deref()
        .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()));

    if let Some(result) = &job.result {
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                let has_structured =
                    result.title.is_some() || result.status.is_some() || result.details.is_some();

                if has_structured {
                    if let Some(title) = &result.title {
                        ui.label(RichText::new(title).monospace().color(TEXT_PRIMARY));
                    }

                    if let Some(details) = &result.details {
                        ui.add_space(4.0);
                        ui.label(RichText::new(details).color(TEXT_DIM));
                    }

                    if let Some(summary) = &result.summary {
                        if !summary.is_empty() {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                            ui.label(RichText::new(summary).color(TEXT_DIM).small());
                        }
                    }

                    ui.add_space(8.0);
                    render_stats_bar(ui, job, result);
                }

                // Render structured output (findings, memory) if available
                if let Some(ref structured) = job.structured_output {
                    if has_structured {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                    let formatted = format_structured_output(structured);
                    if !formatted.is_empty() {
                        render_markdown_scroll(ui, &formatted, commonmark_cache);
                    }
                } else if let Some(text) = response_text {
                    // Fallback: show raw response text
                    if has_structured {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                    ui.add_space(4.0);

                    render_markdown_scroll(ui, text, commonmark_cache);

                    // Still show stats if we didn't render the structured stats bar
                    if !has_structured {
                        if let Some(stats) = &job.stats {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if stats.files_changed > 0 {
                                    ui.label(
                                        RichText::new(format!("{} files", stats.files_changed))
                                            .color(TEXT_MUTED),
                                    );
                                    ui.add_space(8.0);
                                }
                                if let Some(duration) = job.duration_string() {
                                    ui.label(RichText::new(duration).color(TEXT_MUTED));
                                }
                            });
                        }
                    }
                }
            });
    } else if job.structured_output.is_some() || response_text.is_some() {
        // No parsed result, but we have structured output or raw response to display.
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                if let Some(ref structured) = job.structured_output {
                    let formatted = format_structured_output(structured);
                    if !formatted.is_empty() {
                        render_markdown_scroll(ui, &formatted, commonmark_cache);
                    }
                } else if let Some(text) = response_text {
                    ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                    ui.add_space(4.0);
                    render_markdown_scroll(ui, text, commonmark_cache);
                }
            });
    } else if let Some(stats) = &job.stats {
        // Show just stats if no result block but we have timing/files
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if stats.files_changed > 0 {
                ui.label(
                    RichText::new(format!("{} files changed", stats.files_changed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
            if let Some(duration) = job.duration_string() {
                ui.label(RichText::new(format!("⏱ {}", duration)).color(TEXT_MUTED));
            }
        });
    }
}

/// Format SDK structured output (findings, memory) as readable markdown
fn format_structured_output(value: &serde_json::Value) -> String {
    let mut output = String::new();

    // Format findings
    if let Some(findings) = value.get("findings").and_then(|f| f.as_array()) {
        if !findings.is_empty() {
            output.push_str("## Findings\n\n");
            for finding in findings {
                format_finding(&mut output, finding);
            }
        }
    }

    // Format memory entries
    if let Some(memory) = value.get("memory").and_then(|m| m.as_array()) {
        if !memory.is_empty() {
            if !output.is_empty() {
                output.push_str("\n---\n\n");
            }
            output.push_str("## Memory\n\n");
            for entry in memory {
                format_memory_entry(&mut output, entry);
            }
        }
    }

    // Format summary if present
    if let Some(summary) = value.get("summary").and_then(|s| s.as_str()) {
        if !summary.is_empty() {
            if !output.is_empty() {
                output.push_str("\n---\n\n");
            }
            output.push_str("## Summary\n\n");
            output.push_str(summary);
            output.push('\n');
        }
    }

    // Format state if present
    if let Some(state) = value.get("state").and_then(|s| s.as_str()) {
        if !state.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("**State:** {}\n", state));
        }
    }

    output
}

/// Format a single finding as markdown
fn format_finding(output: &mut String, finding: &serde_json::Value) {
    let title = finding.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled Finding");
    let severity = finding.get("severity").and_then(|s| s.as_str());
    let confidence = finding.get("confidence").and_then(|c| c.as_str());

    // Title with severity badge
    output.push_str("### ");
    if let Some(sev) = severity {
        let badge = match sev {
            "critical" => "🔴",
            "high" => "🟠",
            "medium" => "🟡",
            "low" => "🟢",
            "info" => "🔵",
            _ => "⚪",
        };
        output.push_str(&format!("{} [{}] ", badge, sev.to_uppercase()));
    }
    output.push_str(title);
    output.push('\n');

    if let Some(conf) = confidence {
        output.push_str(&format!("**Confidence:** {}\n", conf));
    }

    // Attack scenario
    if let Some(attack) = finding.get("attack_scenario").and_then(|a| a.as_str()) {
        if !attack.is_empty() {
            output.push_str("\n**Attack Scenario:**\n");
            output.push_str(attack);
            output.push('\n');
        }
    }

    // Preconditions
    if let Some(precond) = finding.get("preconditions").and_then(|p| p.as_str()) {
        if !precond.is_empty() {
            output.push_str("\n**Preconditions:** ");
            output.push_str(precond);
            output.push('\n');
        }
    }

    // Reachability
    if let Some(reach) = finding.get("reachability").and_then(|r| r.as_str()) {
        output.push_str(&format!("**Reachability:** {}\n", reach));
    }

    // Impact
    if let Some(impact) = finding.get("impact").and_then(|i| i.as_str()) {
        if !impact.is_empty() {
            output.push_str("\n**Impact:**\n");
            output.push_str(impact);
            output.push('\n');
        }
    }

    // CWE
    if let Some(cwe) = finding.get("cwe_id").and_then(|c| c.as_str()) {
        if !cwe.is_empty() {
            output.push_str(&format!("**CWE:** {}\n", cwe));
        }
    }

    // Affected assets
    if let Some(assets) = finding.get("affected_assets").and_then(|a| a.as_array()) {
        if !assets.is_empty() {
            output.push_str("\n**Affected Assets:**\n");
            for asset in assets {
                if let Some(a) = asset.as_str() {
                    output.push_str(&format!("- `{}`\n", a));
                }
            }
        }
    }

    output.push('\n');
}

/// Format a single memory entry as markdown
fn format_memory_entry(output: &mut String, entry: &serde_json::Value) {
    let mem_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("note");
    let title = entry.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled");
    let confidence = entry.get("confidence").and_then(|c| c.as_str());

    // Icon based on type
    let icon = match mem_type {
        "source" => "📥",
        "sink" => "📤",
        "dataflow" => "➡️",
        "context" => "📋",
        "note" => "📝",
        _ => "•",
    };

    output.push_str(&format!("{} **{}** ", icon, mem_type.to_uppercase()));
    output.push_str(title);

    if let Some(conf) = confidence {
        output.push_str(&format!(" [{}]", conf));
    }
    output.push('\n');

    // Location
    if let Some(file) = entry.get("file").and_then(|f| f.as_str()) {
        let line = entry.get("line").and_then(|l| l.as_u64());
        if let Some(l) = line {
            output.push_str(&format!("  `{}:{}`", file, l));
        } else {
            output.push_str(&format!("  `{}`", file));
        }
        if let Some(symbol) = entry.get("symbol").and_then(|s| s.as_str()) {
            output.push_str(&format!(" ({})", symbol));
        }
        output.push('\n');
    }

    // Dataflow edges (for type=dataflow)
    if mem_type == "dataflow" {
        let from_file = entry.get("from_file").and_then(|f| f.as_str());
        let from_line = entry.get("from_line").and_then(|l| l.as_u64());
        let to_file = entry.get("to_file").and_then(|f| f.as_str());
        let to_line = entry.get("to_line").and_then(|l| l.as_u64());

        if from_file.is_some() || to_file.is_some() {
            output.push_str("  ");
            if let Some(ff) = from_file {
                if let Some(fl) = from_line {
                    output.push_str(&format!("`{}:{}`", ff, fl));
                } else {
                    output.push_str(&format!("`{}`", ff));
                }
            }
            output.push_str(" → ");
            if let Some(tf) = to_file {
                if let Some(tl) = to_line {
                    output.push_str(&format!("`{}:{}`", tf, tl));
                } else {
                    output.push_str(&format!("`{}`", tf));
                }
            }
            output.push('\n');
        }
    }

    // Content
    if let Some(content) = entry.get("content").and_then(|c| c.as_str()) {
        if !content.is_empty() {
            output.push_str("  ");
            output.push_str(content);
            output.push('\n');
        }
    }

    // Tags
    if let Some(tags) = entry.get("tags").and_then(|t| t.as_array()) {
        if !tags.is_empty() {
            output.push_str("  Tags: ");
            let tag_strs: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
            output.push_str(&tag_strs.join(", "));
            output.push('\n');
        }
    }

    output.push('\n');
}

fn render_stats_bar(ui: &mut egui::Ui, job: &Job, result: &crate::JobResult) {
    ui.horizontal(|ui| {
        if let Some(status) = &result.status {
            let result_status_color = match status.as_str() {
                "success" => ACCENT_GREEN,
                "partial" => STATUS_RUNNING,
                "failed" => ACCENT_RED,
                _ => TEXT_MUTED,
            };
            ui.label(RichText::new(format!("● {}", status)).color(result_status_color));
            ui.add_space(8.0);
        }

        if let Some(stats) = &job.stats {
            if stats.files_changed > 0 {
                ui.label(RichText::new(format!("{} files", stats.files_changed)).color(TEXT_MUTED));
                ui.add_space(8.0);
            }
            if stats.lines_added > 0 || stats.lines_removed > 0 {
                ui.label(
                    RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
        }

        if let Some(duration) = job.duration_string() {
            ui.label(RichText::new(duration).color(TEXT_MUTED));
        }
    });
}
