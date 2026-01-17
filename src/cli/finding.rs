//! CLI commands for managing security findings (BugBounty Kanban)

use anyhow::{bail, Context, Result};

use crate::bugbounty::{
    BugBountyJob, BugBountyManager, Confidence, Finding, FindingStatus, Severity,
};
use std::path::Path;

/// List findings
pub fn list(
    project: Option<String>,
    status: Option<String>,
    severity: Option<String>,
    search: Option<String>,
    json: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let mut findings = if let Some(ref project_id) = project {
        manager.list_findings_by_project(project_id)?
    } else if let Some(ref status_str) = status {
        // Slightly faster path for "status-only" queries across projects.
        let status = FindingStatus::from_str(status_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid status: {}", status_str))?;
        manager.list_findings_by_status(status)?
    } else {
        // List all findings from all projects
        let projects = manager.list_projects()?;
        let mut all_findings = Vec::new();
        for p in projects {
            all_findings.extend(manager.list_findings_by_project(&p.id)?);
        }
        all_findings
    };

    // Filter by status if specified (supports project+status)
    if let Some(ref status_str) = status {
        let status = FindingStatus::from_str(status_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid status: {}", status_str))?;
        findings.retain(|f| f.status == status);
    }

    // Filter by severity if specified
    if let Some(ref sev_str) = severity {
        let sev = Severity::from_str(sev_str);
        findings.retain(|f| f.severity == sev);
    }

    // Search filter
    if let Some(query) = search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let query_lower = query.to_lowercase();
        findings.retain(|f| {
            let matches_text = |s: &str| s.to_lowercase().contains(&query_lower);

            matches_text(&f.id)
                || matches_text(&f.project_id)
                || matches_text(&f.title)
                || f.attack_scenario.as_deref().is_some_and(matches_text)
                || f.preconditions.as_deref().is_some_and(matches_text)
                || f.impact.as_deref().is_some_and(matches_text)
                || f.cwe_id.as_deref().is_some_and(matches_text)
                || f.taint_path.as_deref().is_some_and(matches_text)
                || f.affected_assets.iter().any(|a| matches_text(a))
        });
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&findings)?);
    } else {
        if findings.is_empty() {
            println!("No findings found.");
            return Ok(());
        }

        // Print table header
        println!(
            "{:<12} {:<10} {:<12} {:<40} {:<20}",
            "ID", "SEVERITY", "STATUS", "TITLE", "PROJECT"
        );
        println!("{}", "-".repeat(94));

        for f in findings {
            println!(
                "{:<12} {:<10} {:<12} {:<40} {:<20}",
                f.id,
                f.severity.map(|s| s.as_str()).unwrap_or("-"),
                f.status.as_str(),
                truncate(&f.title, 38),
                truncate(&f.project_id, 18),
            );
        }
    }

    Ok(())
}

/// Show a finding by ID
pub fn show(id: &str, json: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let finding = manager
        .get_finding(id)?
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {}", id))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&finding)?);
    } else {
        println!("ID:            {}", finding.id);
        println!("Project:       {}", finding.project_id);
        println!("Title:         {}", finding.title);
        println!(
            "Severity:      {}",
            finding.severity.map(|s| s.as_str()).unwrap_or("-")
        );
        println!("Status:        {}", finding.status.as_str());
        println!(
            "Confidence:    {}",
            finding.confidence.map(|c| c.as_str()).unwrap_or("-")
        );
        println!(
            "Reachability:  {}",
            finding.reachability.map(|r| r.as_str()).unwrap_or("-")
        );

        if let Some(ref scenario) = finding.attack_scenario {
            println!("\nAttack Scenario:");
            println!("  {}", scenario);
        }

        if let Some(ref preconditions) = finding.preconditions {
            println!("\nPreconditions:");
            println!("  {}", preconditions);
        }

        if let Some(ref impact) = finding.impact {
            println!("\nImpact:");
            println!("  {}", impact);
        }

        if !finding.affected_assets.is_empty() {
            println!("\nAffected Assets:");
            for asset in &finding.affected_assets {
                println!("  - {}", asset);
            }
        }

        if let Some(ref cwe) = finding.cwe_id {
            println!("\nCWE: {}", cwe);
        }

        if let Some(ref taint_path) = finding.taint_path {
            println!("\nTaint Path:");
            println!("  {}", taint_path);
        }

        if finding.status == FindingStatus::FalsePositive {
            if let Some(ref reason) = finding.fp_reason {
                println!("\nFP Reason: {}", reason);
            }
        }

        // Show timestamps
        let created = chrono::DateTime::from_timestamp_millis(finding.created_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        let updated = chrono::DateTime::from_timestamp_millis(finding.updated_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("\nCreated: {}  Updated: {}", created, updated);

        // Linked jobs (best-effort)
        if let Ok(job_ids) = manager.job_findings().list_jobs_for_finding(id) {
            if !job_ids.is_empty() {
                let mut jobs: Vec<BugBountyJob> = Vec::new();
                for job_id in job_ids {
                    if let Ok(Some(job)) = manager.jobs().get(&job_id) {
                        jobs.push(job);
                    }
                }
                jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                if !jobs.is_empty() {
                    println!("\nLinked Jobs ({}):", jobs.len());
                    for job in jobs.iter().take(10) {
                        let id_display = format_job_id(job);
                        let mode = job.mode.as_deref().unwrap_or("-");
                        let result_state = job.result_state.as_deref().unwrap_or("-");
                        println!(
                            "  {} [{}] {} ({})",
                            id_display, job.status, mode, result_state
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Create a new finding
#[allow(clippy::too_many_arguments)]
pub fn create(
    work_dir: &Path,
    title: &str,
    project_id: &str,
    severity: Option<String>,
    attack_scenario: Option<String>,
    preconditions: Option<String>,
    impact: Option<String>,
    confidence: Option<String>,
    cwe: Option<String>,
    assets: Vec<String>,
    write_notes: bool,
    json: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check project exists
    if manager.get_project(project_id)?.is_none() {
        bail!(
            "Project '{}' not found. Create it first with: kyco project init --id {} --path <path>",
            project_id,
            project_id
        );
    }

    // Generate next finding ID
    let number = manager.next_finding_number(project_id)?;
    let id = Finding::generate_id(project_id, number);

    let mut finding = Finding::new(&id, project_id, title);

    if let Some(ref s) = severity {
        if let Some(sev) = Severity::from_str(s) {
            finding = finding.with_severity(sev);
        } else {
            bail!("Invalid severity: {}. Use: critical, high, medium, low, info", s);
        }
    }

    if let Some(ref s) = attack_scenario {
        finding = finding.with_attack_scenario(s);
    }

    if let Some(ref s) = preconditions {
        finding.preconditions = Some(s.clone());
    }

    if let Some(ref s) = impact {
        finding = finding.with_impact(s);
    }

    if let Some(ref c) = confidence {
        if let Some(conf) = Confidence::from_str(c) {
            finding = finding.with_confidence(conf);
        } else {
            bail!("Invalid confidence: {}. Use: high, medium, low", c);
        }
    }

    if let Some(ref c) = cwe {
        finding = finding.with_cwe(c);
    }

    for asset in assets {
        finding = finding.with_affected_asset(asset);
    }

    manager.create_finding(&finding)?;

    let notes_result = if write_notes {
        Some(write_notes_file(&manager, work_dir, &finding, false, false)?)
    } else {
        None
    };

    if json {
        // If we wrote notes, source_file might have been updated post-create.
        let finding = manager
            .get_finding(&finding.id)?
            .unwrap_or_else(|| finding.clone());
        println!("{}", serde_json::to_string_pretty(&finding)?);
    } else {
        println!("Created finding: {}", finding.id);
        println!("Status: {} (raw)", finding.status.as_str());
        if let Some(result) = notes_result {
            println!("Notes:  {}", result.path.display());
        }
    }

    Ok(())
}

/// Set the status of a finding (Kanban column change)
pub fn set_status(id: &str, status_str: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let status = FindingStatus::from_str(status_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid status: '{}'. Valid values: raw, needs_repro, verified, report_draft, submitted, triaged, accepted, paid, duplicate, wont_fix, false_positive, out_of_scope",
            status_str
        )
    })?;

    // Check finding exists
    if manager.get_finding(id)?.is_none() {
        bail!("Finding not found: {}", id);
    }

    manager.set_finding_status(id, status)?;
    println!("Updated {} -> {}", id, status.as_str());

    Ok(())
}

/// Mark a finding as false positive
pub fn mark_fp(id: &str, reason: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check finding exists
    if manager.get_finding(id)?.is_none() {
        bail!("Finding not found: {}", id);
    }

    manager
        .findings()
        .mark_false_positive(id, reason)
        .context("Failed to mark as false positive")?;

    println!("Marked {} as false positive", id);
    println!("Reason: {}", reason);

    Ok(())
}

/// Delete a finding
pub fn delete(id: &str, yes: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check finding exists
    let finding = manager
        .get_finding(id)?
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {}", id))?;

    if !yes {
        println!("Delete finding {} ({})? [y/N]", id, finding.title);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    manager.findings().delete(id)?;
    println!("Deleted: {}", id);

    Ok(())
}

/// Export finding to report format
pub fn export(id: &str, format: &str, output: Option<String>) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let finding = manager
        .get_finding(id)?
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {}", id))?;

    let content = match format {
        "markdown" | "md" => export_markdown(&finding),
        "intigriti" => export_intigriti(&finding),
        "hackerone" | "h1" => export_hackerone(&finding),
        _ => bail!("Unknown format: {}. Use: markdown, intigriti, hackerone", format),
    };

    if let Some(path) = output {
        std::fs::write(&path, &content)?;
        println!("Exported to: {}", path);
    } else {
        println!("{}", content);
    }

    Ok(())
}

// Export formats

fn export_markdown(f: &Finding) -> String {
    let mut s = String::new();

    s.push_str(&format!("# {}: {}\n\n", f.id, f.title));
    s.push_str(&format!(
        "**Severity:** {}  \n",
        f.severity.map(|s| s.as_str()).unwrap_or("-")
    ));
    s.push_str(&format!("**Status:** {}  \n", f.status.as_str()));

    if let Some(ref cwe) = f.cwe_id {
        s.push_str(&format!("**CWE:** {}  \n", cwe));
    }

    s.push_str("\n## Attack Scenario\n\n");
    s.push_str(f.attack_scenario.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n");

    s.push_str("## Preconditions\n\n");
    s.push_str(f.preconditions.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n");

    s.push_str("## Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n");

    if !f.affected_assets.is_empty() {
        s.push_str("## Affected Assets\n\n");
        for asset in &f.affected_assets {
            s.push_str(&format!("- {}\n", asset));
        }
        s.push('\n');
    }

    if let Some(ref taint) = f.taint_path {
        s.push_str("## Flow\n\n");
        s.push_str(&format!("```\n{}\n```\n\n", taint));
    }

    s
}

fn export_intigriti(f: &Finding) -> String {
    // Intigriti uses plain text, minimal formatting
    let mut s = String::new();

    s.push_str(&format!("{}\n\n", f.title));

    s.push_str("Summary\n\n");
    s.push_str(f.attack_scenario.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n");

    s.push_str("Steps to reproduce\n\n");
    s.push_str("1. (Add repro steps here)\n\n");

    s.push_str("Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n");

    if !f.affected_assets.is_empty() {
        s.push_str("Affected endpoints\n\n");
        for asset in &f.affected_assets {
            s.push_str(&format!("    {}\n", asset));
        }
        s.push('\n');
    }

    s
}

fn export_hackerone(f: &Finding) -> String {
    // HackerOne uses markdown but simpler format
    let mut s = String::new();

    s.push_str(&format!("## Summary\n\n{}\n\n", f.title));

    if let Some(ref scenario) = f.attack_scenario {
        s.push_str(&format!("{}\n\n", scenario));
    }

    s.push_str("## Steps To Reproduce\n\n");
    s.push_str("1. (Add repro steps here)\n\n");

    s.push_str("## Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(Describe the impact)"));
    s.push_str("\n\n");

    if !f.affected_assets.is_empty() {
        s.push_str("## Affected Assets\n\n");
        for asset in &f.affected_assets {
            s.push_str(&format!("* {}\n", asset));
        }
    }

    s
}

/// Export a finding to `notes/findings/<id>.md` under the project root
pub fn export_notes(
    work_dir: &Path,
    finding_id: &str,
    dry_run: bool,
    force: bool,
    json_output: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let finding = manager
        .get_finding(finding_id)?
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {}", finding_id))?;

    let result = write_notes_file(&manager, work_dir, &finding, dry_run, force)?;

    if json_output {
        let output = serde_json::json!({
            "finding_id": finding.id,
            "project_id": finding.project_id,
            "path": result.path.display().to_string(),
            "action": result.action,
            "dry_run": dry_run,
            "diff": result.diff,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    match result.action.as_str() {
        "up_to_date" => {
            println!("Notes up to date: {}", result.path.display());
        }
        "created" => {
            println!("Notes created:   {}", result.path.display());
        }
        "updated" => {
            println!("Notes updated:   {}", result.path.display());
        }
        _ => {
            println!("Notes: {}", result.path.display());
        }
    }

    if dry_run {
        if let Some(diff) = result.diff {
            println!("\n{}", diff);
        }
        println!("\n(dry-run)");
    }

    Ok(())
}

/// Extract findings from a completed job's output
pub fn extract_from_job(job_id: u64, project: Option<String>, json_output: bool) -> Result<()> {
    use crate::config::Config;

    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Load HTTP settings from config
    let config = Config::load().ok();
    let port = config.as_ref().map(|c| c.settings.gui.http_port).unwrap_or(9876);
    let token = config.and_then(|c| Some(c.settings.gui.http_token).filter(|t| !t.trim().is_empty()));

    // Get job from the running GUI via local HTTP API
    let url = format!("http://127.0.0.1:{}/ctl/jobs/{}", port, job_id);
    let mut req = ureq::get(&url);
    if let Some(ref t) = token {
        req = req.set("X-KYCO-Token", t);
    }

    let resp = req.call().map_err(|e| match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            anyhow::anyhow!("HTTP {}: {}", code, body)
        }
        ureq::Error::Transport(_) => anyhow::anyhow!("Failed to connect to KYCo GUI. Is it running?"),
    })?;

    let body = resp.into_string().context("Failed to read response")?;
    let job_response: serde_json::Value = serde_json::from_str(&body).context("Failed to parse job")?;
    let job_json = job_response.get("job").unwrap_or(&job_response);

    // Get the full_response field
    let full_response = job_json
        .get("full_response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Job {} has no output (full_response is empty)", job_id))?;

    // Try to resolve BugBounty metadata from the DB job registry (preferred, includes stable job id)
    let bb_job = manager.jobs().get_by_kyco_job_id(job_id).ok().flatten();

    // Determine project ID
    let project_id = project
        .or_else(|| bb_job.as_ref().and_then(|j| j.project_id.clone()))
        .or_else(|| {
            job_json
                .get("bugbounty_project_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| anyhow::anyhow!(
            "No project specified and job {} has no bugbounty_project_id. Use --project to specify.",
            job_id
        ))?;

    // Check project exists
    if manager.get_project(&project_id)?.is_none() {
        bail!(
            "Project '{}' not found. Create it first with: kyco project init --id {} --path <path>",
            project_id,
            project_id
        );
    }

    // Process the output (link to the BugBounty job id if known)
    let fallback_job_id = job_id.to_string();
    let bb_job_id = bb_job.as_ref().map(|j| j.id.as_str()).unwrap_or(&fallback_job_id);

    match manager.process_agent_output(&project_id, full_response, Some(bb_job_id))? {
        Some(finding_ids) => {
            if json_output {
                let output = serde_json::json!({
                    "job_id": job_id,
                    "project_id": project_id,
                    "findings_extracted": finding_ids.len(),
                    "finding_ids": finding_ids,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("Extracted {} findings from job #{}", finding_ids.len(), job_id);
                for id in &finding_ids {
                    println!("  Created: {}", id);
                }
            }
        }
        None => {
            if json_output {
                let output = serde_json::json!({
                    "job_id": job_id,
                    "project_id": project_id,
                    "findings_extracted": 0,
                    "finding_ids": [],
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("No findings found in job #{} output", job_id);
                println!("Tip: The job output should contain a ```yaml next_context block");
            }
        }
    }

    Ok(())
}

/// Import findings from tool output (SARIF/Semgrep/Snyk/Nuclei)
pub fn import(path: &str, project: &str, format: &str, json_output: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check project exists
    if manager.get_project(project)?.is_none() {
        bail!(
            "Project '{}' not found. Create it first with: kyco project init --id {} --path <path>",
            project,
            project
        );
    }

    let path = std::path::Path::new(path);
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    let result = match format {
        "sarif" => manager.import_sarif(path, project)?,
        "semgrep" => manager.import_semgrep(path, project)?,
        "nuclei" => manager.import_nuclei(path, project)?,
        "snyk" => manager.import_snyk(path, project)?,
        "auto" => manager.import_auto(path, project)?,
        _ => bail!(
            "Unknown format: {}. Use: sarif, semgrep, snyk, nuclei, auto",
            format
        ),
    };

    if json_output {
        let output = serde_json::json!({
            "findings_count": result.findings.len(),
            "flow_edges_count": result.flow_edges.len(),
            "skipped": result.skipped,
            "warnings": result.warnings,
            "finding_ids": result.findings.iter().map(|f| &f.id).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", result.summary());

        if !result.findings.is_empty() {
            println!("\nImported findings:");
            for finding in &result.findings {
                println!(
                    "  {} [{}] {}",
                    finding.id,
                    finding.severity.map(|s| s.as_str()).unwrap_or("-"),
                    truncate(&finding.title, 50)
                );
            }
        }

        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &result.warnings {
                println!("  - {}", warning);
            }
        }
    }

    Ok(())
}

/// Import/sync findings from `notes/findings/*.md` for a project
pub fn import_notes(work_dir: &Path, project: &str, dry_run: bool, json_output: bool) -> Result<()> {
    use crate::bugbounty::notes::{discover_note_files, parse_note_finding};

    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project_row = manager
        .get_project(project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let root_path = Path::new(&project_row.root_path);
    let project_root = if root_path.is_absolute() {
        root_path.to_path_buf()
    } else {
        work_dir.join(root_path)
    };
    let project_root = project_root.canonicalize().unwrap_or(project_root);

    let files = discover_note_files(&project_root)?;
    if files.is_empty() {
        if json_output {
            let output = serde_json::json!({
                "project_id": project,
                "project_root": project_root.display().to_string(),
                "files_scanned": 0,
                "created": 0,
                "updated": 0,
                "skipped": 0,
                "dry_run": dry_run,
                "warnings": [],
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "No note findings found in `{}`",
                project_root.join("notes/findings").display()
            );
        }
        return Ok(());
    }

    let mut next_number = manager.next_finding_number(project)?;
    let mut created_ids: Vec<String> = Vec::new();
    let mut updated_ids: Vec<String> = Vec::new();
    let mut skipped: usize = 0;
    let mut warnings: Vec<String> = Vec::new();

    for path in files {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => {
                warnings.push(format!("Failed to read {}: {}", path.display(), err));
                continue;
            }
        };

        let parsed = match parse_note_finding(project, &project_root, &path, &content) {
            Ok(p) => p,
            Err(err) => {
                warnings.push(format!("Failed to parse {}: {}", path.display(), err));
                continue;
            }
        };

        let id = parsed.id.clone().unwrap_or_else(|| {
            let id = Finding::generate_id(project, next_number);
            next_number += 1;
            id
        });

        let incoming = parsed.to_finding(project, id.clone());

        if dry_run {
            skipped += 1;
            continue;
        }

        match manager.get_finding(&id)? {
            Some(mut existing) => {
                // Merge only fields that are present in the note (avoid wiping DB-only fields).
                existing.title = incoming.title;

                if parsed.severity.present {
                    existing.severity = incoming.severity;
                }
                if parsed.status.present {
                    existing.status = incoming.status;
                }
                if parsed.attack_scenario.present {
                    existing.attack_scenario = incoming.attack_scenario;
                }
                if parsed.preconditions.present {
                    existing.preconditions = incoming.preconditions;
                }
                if parsed.reachability.present {
                    existing.reachability = incoming.reachability;
                }
                if parsed.impact.present {
                    existing.impact = incoming.impact;
                }
                if parsed.confidence.present {
                    existing.confidence = incoming.confidence;
                }
                if parsed.cwe_id.present {
                    existing.cwe_id = incoming.cwe_id;
                }
                if parsed.cvss_score.present {
                    existing.cvss_score = incoming.cvss_score;
                }
                if parsed.affected_assets.present {
                    existing.affected_assets = incoming.affected_assets;
                }
                if parsed.taint_path.present {
                    existing.taint_path = incoming.taint_path;
                }
                if parsed.notes.present {
                    existing.notes = incoming.notes;
                }
                if parsed.fp_reason.present {
                    existing.fp_reason = incoming.fp_reason;
                }
                if parsed.source_file.present {
                    existing.source_file = incoming.source_file;
                }

                manager.findings().update(&existing)?;
                updated_ids.push(id);
            }
            None => {
                manager.create_finding(&incoming)?;
                created_ids.push(id);
            }
        }
    }

    if json_output {
        let output = serde_json::json!({
            "project_id": project,
            "project_root": project_root.display().to_string(),
            "files_scanned": created_ids.len() + updated_ids.len() + skipped,
            "created": created_ids.len(),
            "updated": updated_ids.len(),
            "skipped": skipped,
            "dry_run": dry_run,
            "created_ids": created_ids,
            "updated_ids": updated_ids,
            "warnings": warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "Notes import for {}: {} created, {} updated{}",
            project,
            created_ids.len(),
            updated_ids.len(),
            if dry_run { " (dry-run)" } else { "" }
        );
        if !warnings.is_empty() {
            println!("\nWarnings:");
            for w in &warnings {
                println!("  - {}", w);
            }
        }
    }

    Ok(())
}

// Helpers

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn format_job_id(job: &BugBountyJob) -> String {
    job.kyco_job_id
        .map(|id| format!("#{}", id))
        .unwrap_or_else(|| job.id.clone())
}

/// Link a finding to a job
pub fn link_job(finding_id: &str, job_id: &str, link_type: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check finding exists
    if manager.get_finding(finding_id)?.is_none() {
        bail!("Finding not found: {}", finding_id);
    }

    // Resolve job ID - support both BugBounty job ID and KYCo job ID (with # prefix)
    let resolved_job_id = resolve_job_id(&manager, job_id)?;

    // Check if already linked
    if manager.job_findings().is_linked(&resolved_job_id, finding_id)? {
        println!("Already linked: {} <-> {}", finding_id, job_id);
        return Ok(());
    }

    // Create link
    manager.job_findings().link(&resolved_job_id, finding_id, link_type)?;
    println!("Linked {} <-> {} (type: {})", finding_id, job_id, link_type);

    Ok(())
}

/// Unlink a finding from a job
pub fn unlink_job(finding_id: &str, job_id: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check finding exists
    if manager.get_finding(finding_id)?.is_none() {
        bail!("Finding not found: {}", finding_id);
    }

    // Resolve job ID
    let resolved_job_id = resolve_job_id(&manager, job_id)?;

    // Remove link
    manager.job_findings().unlink(&resolved_job_id, finding_id)?;
    println!("Unlinked {} <-> {}", finding_id, job_id);

    Ok(())
}

/// Resolve job ID - supports BugBounty job ID or KYCo job ID (with # prefix)
fn resolve_job_id(manager: &BugBountyManager, job_id: &str) -> Result<String> {
    let kyco_job_id = if let Some(kyco_id_str) = job_id.strip_prefix('#') {
        Some(
            kyco_id_str
                .parse::<u64>()
                .with_context(|| format!("Invalid KYCo job ID: {}", job_id))?,
        )
    } else {
        job_id.parse::<u64>().ok()
    };

    if let Some(kyco_id) = kyco_job_id {
        if let Some(bb_job) = manager.jobs().get_by_kyco_job_id(kyco_id)? {
            return Ok(bb_job.id);
        }
        bail!("No BugBounty job found for KYCo job #{}", kyco_id);
    }

    // Otherwise assume it's a BugBounty job ID directly
    // Verify it exists
    if manager.jobs().get(job_id)?.is_none() {
        bail!("BugBounty job not found: {}", job_id);
    }

    Ok(job_id.to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
struct NotesWriteResult {
    path: std::path::PathBuf,
    action: NotesWriteAction,
    diff: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum NotesWriteAction {
    UpToDate,
    Created,
    Updated,
}

impl NotesWriteAction {
    fn as_str(&self) -> &'static str {
        match self {
            NotesWriteAction::UpToDate => "up_to_date",
            NotesWriteAction::Created => "created",
            NotesWriteAction::Updated => "updated",
        }
    }
}

fn resolve_project_root(work_dir: &Path, root_path: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(root_path);
    let abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        work_dir.join(root)
    };
    abs.canonicalize().unwrap_or(abs)
}

fn resolve_notes_file_path(
    project_root: &Path,
    finding: &Finding,
) -> std::path::PathBuf {
    if let Some(ref src) = finding.source_file {
        let p = std::path::PathBuf::from(src);
        if p.is_absolute() {
            return p;
        }
        return project_root.join(p);
    }

    let rel = crate::bugbounty::notes::default_note_rel_path(&finding.id);
    project_root.join(rel)
}

fn system_time_to_millis(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

fn write_notes_file(
    manager: &BugBountyManager,
    work_dir: &Path,
    finding: &Finding,
    dry_run: bool,
    force: bool,
) -> Result<NotesWriteResult> {
    use crate::bugbounty::notes::render_note_markdown;

    let project = manager
        .get_project(&finding.project_id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", finding.project_id))?;
    let project_root = resolve_project_root(work_dir, &project.root_path);
    let path = resolve_notes_file_path(&project_root, finding);

    let new_content = render_note_markdown(finding);
    let old_content = std::fs::read_to_string(&path).ok();

    if old_content.as_deref() == Some(&new_content) {
        return Ok(NotesWriteResult {
            path,
            action: NotesWriteAction::UpToDate,
            diff: None,
        });
    }

    let diff = old_content
        .as_deref()
        .map(|old| render_simple_line_diff(old, &new_content, "existing", "proposed"))
        .or_else(|| Some(render_simple_line_diff("", &new_content, "existing", "proposed")));

    if dry_run {
        return Ok(NotesWriteResult {
            path,
            action: if old_content.is_some() {
                NotesWriteAction::Updated
            } else {
                NotesWriteAction::Created
            },
            diff,
        });
    }

    if !force && path.exists() {
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if let Some(modified_ms) = system_time_to_millis(modified) {
                    if modified_ms > finding.updated_at {
                        anyhow::bail!(
                            "Conflict exporting notes for {}: {} looks newer than the DB (file modified at {}, db updated at {}). Run `kyco finding import-notes --project {}` first, or re-run with --force.",
                            finding.id,
                            path.display(),
                            modified_ms,
                            finding.updated_at,
                            finding.project_id
                        );
                    }
                }
            }
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    std::fs::write(&path, &new_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    // Persist source_file back into the DB for round-trip sync.
    let source_file = path
        .strip_prefix(&project_root)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let mut updated = finding.clone();
    updated.source_file = Some(source_file);
    manager.findings().update(&updated)?;

    Ok(NotesWriteResult {
        path,
        action: if old_content.is_some() {
            NotesWriteAction::Updated
        } else {
            NotesWriteAction::Created
        },
        diff: None,
    })
}

fn render_simple_line_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let m = old_lines.len();
    let n = new_lines.len();

    // LCS DP table (O(m*n)) - fine for small notes files.
    let mut lcs = vec![vec![0usize; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old_lines[i] == new_lines[j] {
                lcs[i][j] = lcs[i + 1][j + 1] + 1;
            } else {
                lcs[i][j] = lcs[i + 1][j].max(lcs[i][j + 1]);
            }
        }
    }

    let mut out = String::new();
    out.push_str(&format!("--- {}\n+++ {}\n", old_label, new_label));

    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        if old_lines[i] == new_lines[j] {
            out.push_str(" ");
            out.push_str(old_lines[i]);
            out.push('\n');
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            out.push_str("-");
            out.push_str(old_lines[i]);
            out.push('\n');
            i += 1;
        } else {
            out.push_str("+");
            out.push_str(new_lines[j]);
            out.push('\n');
            j += 1;
        }
    }

    while i < m {
        out.push_str("-");
        out.push_str(old_lines[i]);
        out.push('\n');
        i += 1;
    }
    while j < n {
        out.push_str("+");
        out.push_str(new_lines[j]);
        out.push('\n');
        j += 1;
    }

    out
}
