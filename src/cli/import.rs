//! Import helper commands (tool aliases).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::bugbounty::{BugBountyManager, Finding, ImportResult};
use crate::cli::job::ctl_create_jobs;

fn load_active_project_id() -> Option<String> {
    let path = dirs::home_dir()?.join(".kyco").join("active_project");
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_project_id(project: Option<String>) -> Result<String> {
    if let Some(id) = project {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(id) = load_active_project_id() {
        return Ok(id);
    }

    bail!(
        "No project specified and no active project selected.\nUse --project <id> or run: kyco project select <id>"
    )
}

fn resolve_project_root(work_dir: &Path, project_root_path: &str) -> PathBuf {
    let raw = PathBuf::from(project_root_path);
    let abs = if raw.is_absolute() { raw } else { work_dir.join(raw) };
    abs.canonicalize().unwrap_or(abs)
}

fn parse_path_and_line(raw: &str) -> Option<(String, Option<usize>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let Some(idx) = trimmed.rfind(':') else {
        return Some((trimmed.to_string(), None));
    };

    let (left, right) = trimmed.split_at(idx);
    let right = &right[1..];
    if right.chars().all(|c| c.is_ascii_digit()) {
        let line: usize = right.parse().ok()?;
        let left = left.trim();
        if !left.is_empty() {
            return Some((left.to_string(), Some(line)));
        }
    }

    Some((trimmed.to_string(), None))
}

fn best_effort_finding_location(
    project_root: &Path,
    finding: &Finding,
) -> Option<(PathBuf, Option<usize>)> {
    for asset in &finding.affected_assets {
        let Some((path_raw, line)) = parse_path_and_line(asset) else {
            continue;
        };

        if path_raw.contains("://") {
            continue;
        }

        let candidate = PathBuf::from(&path_raw);
        let abs = if candidate.is_absolute() {
            candidate
        } else {
            project_root.join(candidate)
        };

        if abs.exists() && abs.is_file() {
            return Some((abs, line));
        }
    }
    None
}

fn default_verify_prompt(tool_name: &str, finding: &Finding, location: Option<(&Path, Option<usize>)>) -> String {
    let sev = finding
        .severity
        .map(|s| s.as_str().to_uppercase())
        .unwrap_or_else(|| "-".to_string());

    let mut lines = Vec::new();
    lines.push("Verify the following imported finding and produce concrete evidence.".to_string());
    lines.push(String::new());
    lines.push(format!("- Imported from: {tool_name}"));
    lines.push(format!("- Finding: {} ({})", finding.id, finding.title));
    lines.push(format!("- Severity: {sev}"));
    lines.push(format!("- Current status: {}", finding.status.as_str()));
    if let Some((path, line)) = location {
        if let Some(line) = line {
            lines.push(format!("- Location: {}:{}", path.display(), line));
        } else {
            lines.push(format!("- Location: {}", path.display()));
        }
    }
    lines.push(String::new());
    lines.push("Goal: confirm exploitability OR explain why it's a false positive.".to_string());
    lines.push("Output: include next_context.findings[] for this finding; add artifacts/flow_edges if relevant.".to_string());
    lines.push("If it is a false positive, set status=false_positive and provide fp_reason.".to_string());
    lines.join("\n")
}

fn print_import_result(result: &ImportResult, json_output: bool) -> Result<()> {
    if json_output {
        let output = serde_json::json!({
            "findings_count": result.findings.len(),
            "flow_edges_count": result.flow_edges.len(),
            "skipped": result.skipped,
            "warnings": result.warnings,
            "finding_ids": result.findings.iter().map(|f| &f.id).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("{}", result.summary());

    if !result.findings.is_empty() {
        println!("\nImported findings:");
        for finding in &result.findings {
            let sev = finding
                .severity
                .map(|s| s.as_str())
                .unwrap_or("-");
            println!("  {} [{}] {}", finding.id, sev, finding.title);
        }
    }

    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  - {}", warning);
        }
    }

    Ok(())
}

pub fn import_tool(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    tool_name: &str,
    file: &str,
    project: Option<String>,
    format: &str,
    create_jobs: bool,
    queue_jobs: bool,
    job_skill: &str,
    agent: Option<String>,
    agents: Vec<String>,
    json_output: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project_id = resolve_project_id(project)?;
    let project_row = manager
        .get_project(&project_id)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project_id))?;

    let input_path = Path::new(file);
    if !input_path.exists() {
        bail!("File not found: {}", input_path.display());
    }

    let import_result = match format {
        "sarif" => manager.import_sarif(input_path, &project_id)?,
        "semgrep" => manager.import_semgrep(input_path, &project_id)?,
        "nuclei" => manager.import_nuclei(input_path, &project_id)?,
        "snyk" => manager.import_snyk(input_path, &project_id)?,
        "auto" => manager.import_auto(input_path, &project_id)?,
        _ => bail!("Unknown import format: {}", format),
    };

    if !create_jobs {
        return print_import_result(&import_result, json_output);
    }

    let project_root = resolve_project_root(work_dir, &project_row.root_path);
    let mut created: Vec<(String, Vec<u64>)> = Vec::new();
    let agents = agents
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    for finding in &import_result.findings {
        let location = best_effort_finding_location(&project_root, finding);
        let (file_path, line_start) = match location.as_ref() {
            Some((path, line)) => (Some(path.display().to_string()), *line),
            None => (None, None),
        };

        let prompt = default_verify_prompt(tool_name, finding, location.as_ref().map(|(p, l)| (p.as_path(), *l)));
        let payload = serde_json::json!({
            "file_path": file_path,
            "line_start": line_start,
            "line_end": line_start,
            "selected_text": null,
            "mode": job_skill,
            "prompt": prompt,
            "bugbounty_project_id": project_id.clone(),
            "bugbounty_finding_ids": [finding.id.clone()],
            "agent": agent.clone(),
            "agents": if agents.is_empty() {
                None::<Vec<String>>
            } else {
                Some(agents.clone())
            },
            "queue": queue_jobs,
            "force_worktree": false,
        });

        let created_jobs = ctl_create_jobs(work_dir, config_override, payload)
            .with_context(|| format!("Failed to create verify job for {}", finding.id))?;
        created.push((finding.id.clone(), created_jobs.job_ids));
    }

    if json_output {
        let output = serde_json::json!({
            "project_id": project_id,
            "tool": tool_name,
            "imported_findings": import_result.findings.len(),
            "finding_ids": import_result.findings.iter().map(|f| &f.id).collect::<Vec<_>>(),
            "create_jobs": true,
            "queue_jobs": queue_jobs,
            "job_skill": job_skill,
            "jobs_created": created.len(),
            "created": created.iter().map(|(fid, jobs)| serde_json::json!({"finding_id": fid, "job_ids": jobs})).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    print_import_result(&import_result, false)?;
    println!("\nCreated {} verify jobs ({}):", created.len(), if queue_jobs { "queued" } else { "pending" });
    for (finding_id, job_ids) in created {
        let ids = job_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {} → {}", finding_id, ids);
    }

    Ok(())
}
