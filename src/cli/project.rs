//! CLI commands for managing BugBounty projects

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::bugbounty::{
    parse_scope_file, BugBountyJob, BugBountyManager, Project, ProjectMetadata, ToolPolicy,
};

/// List all projects
pub fn list(platform: Option<String>, json: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let projects = if let Some(ref plat) = platform {
        manager.projects().list_by_platform(plat)?
    } else {
        manager.list_projects()?
    };

    if json {
        #[derive(serde::Serialize)]
        struct ProjectListItem {
            #[serde(flatten)]
            project: Project,
            jobs_total: usize,
            jobs_open: usize,
            findings_total: usize,
            findings_open: usize,
            last_activity_at: Option<i64>,
        }

        let mut items: Vec<ProjectListItem> = Vec::new();
        for p in projects {
            let jobs = manager.jobs().list_by_project(&p.id)?;
            let jobs_open = jobs
                .iter()
                .filter(|j| j.status != "done" && j.status != "failed")
                .count();
            let last_job_at = jobs
                .iter()
                .filter_map(|j| j.completed_at.or(j.started_at).or(Some(j.created_at)))
                .max();

            let findings = manager.list_findings_by_project(&p.id)?;
            let findings_open = findings.iter().filter(|f| !f.status.is_terminal()).count();
            let last_finding_at = findings
                .iter()
                .map(|f| f.updated_at.max(f.created_at))
                .max();

            let last_activity_at = last_job_at.max(last_finding_at);

            items.push(ProjectListItem {
                project: p,
                jobs_total: jobs.len(),
                jobs_open,
                findings_total: findings.len(),
                findings_open,
                last_activity_at,
            });
        }

        // Most recently active first (None last)
        items.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));

        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        if projects.is_empty() {
            println!("No projects found.");
            println!("Discover projects with: kyco project discover");
            println!("Or create one with: kyco project init --id <id> --root <path>");
            return Ok(());
        }

        println!(
            "{:<30} {:<12} {:<10} {:<12} {:<18} {:<40}",
            "ID", "PLATFORM", "JOBS", "FINDINGS", "LAST ACTIVITY", "PATH"
        );
        println!("{}", "-".repeat(124));

        let mut rows = Vec::new();
        for p in projects {
            let jobs = manager.jobs().list_by_project(&p.id)?;
            let jobs_open = jobs
                .iter()
                .filter(|j| j.status != "done" && j.status != "failed")
                .count();
            let last_job_at = jobs
                .iter()
                .filter_map(|j| j.completed_at.or(j.started_at).or(Some(j.created_at)))
                .max();

            let findings = manager.list_findings_by_project(&p.id)?;
            let findings_open = findings.iter().filter(|f| !f.status.is_terminal()).count();
            let last_finding_at = findings
                .iter()
                .map(|f| f.updated_at.max(f.created_at))
                .max();

            let last_activity_at = last_job_at.max(last_finding_at);

            rows.push((
                last_activity_at,
                p,
                format!("{}/{}", jobs_open, jobs.len()),
                format!("{}/{}", findings_open, findings.len()),
            ));
        }

        rows.sort_by(|a, b| b.0.cmp(&a.0));

        for (last_activity_at, p, jobs_str, findings_str) in rows {
            println!(
                "{:<30} {:<12} {:<10} {:<12} {:<18} {:<40}",
                truncate(&p.id, 28),
                truncate(p.platform.as_deref().unwrap_or("-"), 10),
                jobs_str,
                findings_str,
                format_timestamp(last_activity_at),
                truncate(&p.root_path, 38),
            );
        }
    }

    Ok(())
}

/// Show a project by ID
pub fn show(id: &str, json: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project = manager
        .get_project(id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", id))?;

    let stats = manager.projects().get_stats(id)?;

    if json {
        #[derive(serde::Serialize)]
        struct ProjectWithStats {
            #[serde(flatten)]
            project: Project,
            #[serde(flatten)]
            stats: crate::bugbounty::ProjectStats,
        }
        let output = ProjectWithStats {
            project,
            stats,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("ID:         {}", project.id);
        println!("Path:       {}", project.root_path);
        println!(
            "Platform:   {}",
            project.platform.as_deref().unwrap_or("-")
        );
        println!(
            "Target:     {}",
            project.target_name.as_deref().unwrap_or("-")
        );
        println!(
            "Last act.:  {}",
            format_timestamp(stats.last_activity_at)
        );

        println!("\nFindings:");
        println!("  Total:      {}", stats.findings_total);
        println!("  Raw:        {}", stats.findings_raw);
        println!("  Verified:   {}", stats.findings_verified);
        println!("  Submitted+: {}", stats.findings_submitted);
        println!("  Terminal:   {}", stats.findings_terminal);

        println!("\nJobs:");
        println!("  Total:    {}", stats.jobs_total);
        println!("  Pending:  {}", stats.jobs_pending);
        println!("  Running:  {}", stats.jobs_running);
        println!("  Done:     {}", stats.jobs_done);
        println!("  Failed:   {}", stats.jobs_failed);

        let recent_jobs = manager.jobs().list_recent_by_project(id, 5)?;
        if !recent_jobs.is_empty() {
            println!("\nRecent Jobs:");
            for job in &recent_jobs {
                let id_display = job
                    .kyco_job_id
                    .map(|id| format!("#{}", id))
                    .unwrap_or_else(|| job.id.clone());
                let mode = job.mode.as_deref().unwrap_or("-");
                let result_state = job.result_state.as_deref().unwrap_or("-");
                let when = format_timestamp(job.completed_at.or(job.started_at).or(Some(job.created_at)));
                println!(
                    "  {} [{}] {} ({}) - {}",
                    id_display, job.status, mode, result_state, when
                );
            }
        }

        if let Some(ref scope) = project.scope {
            println!("\nScope:");
            if !scope.in_scope.is_empty() {
                println!("  In-scope:");
                for s in &scope.in_scope {
                    println!("    - {}", s);
                }
            }
            if !scope.out_of_scope.is_empty() {
                println!("  Out-of-scope:");
                for s in &scope.out_of_scope {
                    println!("    - {}", s);
                }
            }
            if let Some(rate) = scope.rate_limit {
                println!("  Rate limit: {} req/s", rate);
            }
        }

        if let Some(ref policy) = project.tool_policy {
            if !policy.blocked_commands.is_empty() {
                println!("\nBlocked commands:");
                for cmd in &policy.blocked_commands {
                    println!("  - {}", cmd);
                }
            }
            if let Some(ref wrapper) = policy.network_wrapper {
                println!("Network wrapper: {}", wrapper);
            }
        }

        if let Some(ref meta) = project.metadata {
            if !meta.stack.is_empty() {
                println!("\nStack:");
                for s in &meta.stack {
                    println!("  - {}", s);
                }
            }
            if let Some(ref auth) = meta.auth_notes {
                if !auth.trim().is_empty() {
                    println!("\nAuth notes:\n{}", auth.trim());
                }
            }
            if !meta.endpoints.is_empty() {
                println!("\nEndpoints:");
                for e in &meta.endpoints {
                    println!("  - {}", e);
                }
            }
            if !meta.links.is_empty() {
                println!("\nLinks:");
                for l in &meta.links {
                    println!("  - {}", l);
                }
            }
        }

        // Show timestamps
        let created = chrono::DateTime::from_timestamp_millis(project.created_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("\nCreated: {}", created);
    }

    Ok(())
}

/// Discover projects from BugBounty/programs/ directory
pub fn discover(path: Option<String>, dry_run: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let base_path = path.unwrap_or_else(|| ".".to_string());
    let base = Path::new(&base_path);
    let base_abs = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());

    // Look for BugBounty/programs/*/ or programs/*/
    let patterns = [
        base.join("BugBounty/programs"),
        base.join("programs"),
    ];

    let mut discovered: Vec<(String, std::path::PathBuf, Option<String>, Option<String>)> =
        Vec::new();

    for pattern_base in &patterns {
        if !pattern_base.exists() {
            continue;
        }

        // List directories in programs/
        if let Ok(entries) = std::fs::read_dir(pattern_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Check if it looks like a project (has scope.md or CLAUDE.md)
                let has_scope = path.join("scope.md").exists();
                let has_claude = path.join("CLAUDE.md").exists();

                if has_scope || has_claude {
                    let dir_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    // Parse platform-target from directory name
                    let (platform, target) = if let Some((p, t)) = dir_name.split_once('-') {
                        (Some(p.to_string()), Some(t.to_string()))
                    } else {
                        (None, Some(dir_name.to_string()))
                    };

                    discovered.push((dir_name.to_string(), path.clone(), platform, target));
                }
            }
        }
    }

    if discovered.is_empty() {
        println!("No projects discovered.");
        println!("Looking in: {:?}", patterns);
        println!("\nExpected structure:");
        println!("  BugBounty/programs/<platform>-<target>/");
        println!("  with scope.md or CLAUDE.md file");
        return Ok(());
    }

    println!("Discovered {} project(s):\n", discovered.len());

    for (id, path, platform, _target) in &discovered {
        println!(
            "  {} ({}) -> {}",
            id,
            platform.as_deref().unwrap_or("unknown"),
            path.display()
        );
    }

    if dry_run {
        println!("\nDry run - no projects created.");
        return Ok(());
    }

    println!();

    // Create projects
    let mut created = 0;
    let mut skipped = 0;

    for (id, path, platform, target) in discovered {
        // Check if already exists
        if manager.get_project(&id)?.is_some() {
            println!("  Skipped {} (already exists)", id);
            skipped += 1;
            continue;
        }

        let root_path = normalize_root_path(&base_abs, &path);
        let mut project = Project::new(&id, root_path);
        if let Some(p) = platform {
            project = project.with_platform(p);
        }
        if let Some(t) = target {
            project = project.with_target_name(t);
        }

        if let Some(scope) = load_project_scope(&path) {
            project.scope = Some(scope);
        }
        if let Some(policy) = infer_tool_policy(&path) {
            project.tool_policy = Some(policy);
        }
        if let Some(metadata) = infer_project_metadata(&path) {
            project.metadata = Some(metadata);
        }

        manager.create_project(&project)?;
        println!("  Created {}", id);
        created += 1;
    }

    println!("\nCreated: {}  Skipped: {}", created, skipped);

    Ok(())
}

/// Select a project as the active project
pub fn select(id: &str) -> Result<()> {
    let manager = BugBountyManager::new()?;
    if manager.get_project(id)?.is_none() {
        bail!("Project not found: {}", id);
    }

    // Persist the selection
    if let Some(home) = dirs::home_dir() {
        let kyco_dir = home.join(".kyco");
        let _ = std::fs::create_dir_all(&kyco_dir);
        let path = kyco_dir.join("active_project");
        std::fs::write(&path, id)?;
    }

    println!("Selected project: {}", id);
    Ok(())
}

/// Initialize a new project
pub fn init(id: &str, path: &str, platform: Option<String>) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Check if already exists
    if manager.get_project(id)?.is_some() {
        bail!("Project '{}' already exists", id);
    }

    let mut project = Project::new(id, path);

    // Derive platform/target from ID if not specified
    if let Some(p) = platform {
        project = project.with_platform(p);
    } else {
        project = project.derive_from_id();
    }

    let project_dir = Path::new(path);
    if let Some(scope) = load_project_scope(project_dir) {
        project.scope = Some(scope);
    }
    if let Some(policy) = infer_tool_policy(project_dir) {
        project.tool_policy = Some(policy);
    }

    manager.create_project(&project)?;

    println!("Created project: {}", id);
    println!("Path: {}", path);
    if let Some(ref p) = project.platform {
        println!("Platform: {}", p);
    }

    Ok(())
}

/// Delete a project
pub fn delete(id: &str, yes: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project = manager
        .get_project(id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", id))?;

    // Check for existing findings
    let findings = manager.list_findings_by_project(id)?;
    if !findings.is_empty() && !yes {
        println!(
            "Warning: Project '{}' has {} finding(s).",
            id,
            findings.len()
        );
    }

    if !yes {
        println!("Delete project {} ({})? [y/N]", id, project.root_path);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    manager.projects().delete(id)?;
    println!("Deleted project: {}", id);

    if !findings.is_empty() {
        println!("Note: {} finding(s) were also deleted.", findings.len());
    }

    Ok(())
}

/// Generate project overview
pub fn overview(
    project: Option<String>,
    output: Option<String>,
    update_global: bool,
    json: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let projects = if let Some(ref pid) = project {
        let p = manager
            .get_project(pid)?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", pid))?;
        vec![p]
    } else {
        manager.list_projects()?
    };

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    let content = if json {
        generate_overview_json(&manager, &projects)?
    } else {
        generate_overview_markdown(&manager, &projects)?
    };

    // Write output
    if let Some(path) = output {
        std::fs::write(&path, &content)?;
        println!("Overview written to: {}", path);
    } else {
        println!("{}", content);
    }

    // Optionally update global overview
    if update_global {
        let global_path = "BugBounty/OVERVIEW.md";
        if std::path::Path::new("BugBounty").exists() {
            std::fs::write(global_path, generate_overview_markdown(&manager, &projects)?)?;
            println!("Updated: {}", global_path);
        }
    }

    Ok(())
}

fn generate_overview_markdown(
    manager: &BugBountyManager,
    projects: &[Project],
) -> Result<String> {
    use crate::bugbounty::FindingStatus;

    let mut md = String::new();

    md.push_str("# BugBounty Overview\n\n");
    md.push_str(&format!(
        "_Generated: {}_\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));

    // Summary table
    md.push_str("## Summary\n\n");
    md.push_str("| Project | Platform | Jobs | Findings | Raw | Verified | Submitted |\n");
    md.push_str("|---------|----------|------|----------|-----|----------|----------|\n");

    let mut total_findings = 0;
    let mut total_raw = 0;
    let mut total_verified = 0;
    let mut total_submitted = 0;
    let mut total_jobs = 0;

    for p in projects {
        let jobs = manager.jobs().list_by_project(&p.id)?;
        let findings = manager.list_findings_by_project(&p.id)?;
        let raw = findings.iter().filter(|f| f.status == FindingStatus::Raw).count();
        let verified = findings.iter().filter(|f| f.status == FindingStatus::Verified).count();
        let submitted = findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Submitted | FindingStatus::Triaged | FindingStatus::Accepted | FindingStatus::Paid))
            .count();

        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            p.id,
            p.platform.as_deref().unwrap_or("-"),
            jobs.len(),
            findings.len(),
            raw,
            verified,
            submitted
        ));

        total_findings += findings.len();
        total_raw += raw;
        total_verified += verified;
        total_submitted += submitted;
        total_jobs += jobs.len();
    }

    md.push_str(&format!(
        "| **Total** | | **{}** | **{}** | **{}** | **{}** | **{}** |\n\n",
        total_jobs, total_findings, total_raw, total_verified, total_submitted
    ));

    // Per-project details
    md.push_str("## Projects\n\n");

    for p in projects {
        let jobs = manager.jobs().list_by_project(&p.id)?;
        let findings = manager.list_findings_by_project(&p.id)?;

        md.push_str(&format!("### {}\n\n", p.id));
        md.push_str(&format!("- **Path:** {}\n", p.root_path));
        if let Some(ref platform) = p.platform {
            md.push_str(&format!("- **Platform:** {}\n", platform));
        }
        if !jobs.is_empty() {
            let pending = jobs.iter().filter(|j| j.status == "pending").count();
            let running = jobs.iter().filter(|j| j.status == "running").count();
            let done = jobs.iter().filter(|j| j.status == "done").count();
            let failed = jobs.iter().filter(|j| j.status == "failed").count();
            md.push_str(&format!(
                "- **Jobs:** {} (pending {}, running {}, done {}, failed {})\n",
                jobs.len(),
                pending,
                running,
                done,
                failed
            ));
        } else {
            md.push_str("- **Jobs:** 0\n");
        }
        md.push_str(&format!("- **Findings:** {}\n\n", findings.len()));

        if !jobs.is_empty() {
            md.push_str("**Recent Jobs:**\n\n");
            for job in jobs.iter().take(5) {
                md.push_str(&format!(
                    "- {} [{}] {} ({})\n",
                    format_job_id(job),
                    job.status,
                    job.mode.as_deref().unwrap_or("-"),
                    job.result_state.as_deref().unwrap_or("-"),
                ));
            }
            md.push_str("\n");
        }

        if !findings.is_empty() {
            // Group by severity
            let critical: Vec<_> = findings
                .iter()
                .filter(|f| f.severity == Some(crate::bugbounty::Severity::Critical))
                .collect();
            let high: Vec<_> = findings
                .iter()
                .filter(|f| f.severity == Some(crate::bugbounty::Severity::High))
                .collect();

            if !critical.is_empty() || !high.is_empty() {
                md.push_str("**High-Priority Findings:**\n\n");
                for f in critical.iter().chain(high.iter()) {
                    let sev = f.severity.map(|s| s.as_str()).unwrap_or("-");
                    md.push_str(&format!(
                        "- **{}** [{}] {} ({})\n",
                        f.id,
                        sev.to_uppercase(),
                        f.title,
                        f.status.as_str()
                    ));
                }
                md.push_str("\n");
            }

            // Actionable items
            let actionable: Vec<_> = findings
                .iter()
                .filter(|f| f.status.is_actionable())
                .collect();
            if !actionable.is_empty() {
                md.push_str(&format!("**Actionable:** {} findings need attention\n\n", actionable.len()));
            }
        }
    }

    Ok(md)
}

fn generate_overview_json(
    manager: &BugBountyManager,
    projects: &[Project],
) -> Result<String> {
    use crate::bugbounty::FindingStatus;

    #[derive(serde::Serialize)]
    struct Overview {
        generated_at: String,
        total_projects: usize,
        total_jobs: usize,
        total_findings: usize,
        findings_by_status: std::collections::HashMap<String, usize>,
        jobs_by_status: std::collections::HashMap<String, usize>,
        projects: Vec<ProjectOverview>,
    }

    #[derive(serde::Serialize)]
    struct ProjectOverview {
        id: String,
        platform: Option<String>,
        root_path: String,
        jobs_count: usize,
        jobs_pending: usize,
        jobs_running: usize,
        jobs_done: usize,
        jobs_failed: usize,
        last_job_at: Option<i64>,
        findings_count: usize,
        raw_count: usize,
        verified_count: usize,
        submitted_count: usize,
    }

    let mut overview = Overview {
        generated_at: chrono::Utc::now().to_rfc3339(),
        total_projects: projects.len(),
        total_jobs: 0,
        total_findings: 0,
        findings_by_status: std::collections::HashMap::new(),
        jobs_by_status: std::collections::HashMap::new(),
        projects: Vec::new(),
    };

    for p in projects {
        let jobs = manager.jobs().list_by_project(&p.id)?;
        let pending = jobs.iter().filter(|j| j.status == "pending").count();
        let running = jobs.iter().filter(|j| j.status == "running").count();
        let done = jobs.iter().filter(|j| j.status == "done").count();
        let failed = jobs.iter().filter(|j| j.status == "failed").count();
        let last_job_at = jobs
            .iter()
            .filter_map(|j| j.completed_at.or(j.started_at).or(Some(j.created_at)))
            .max();

        overview.total_jobs += jobs.len();
        for job in &jobs {
            *overview
                .jobs_by_status
                .entry(job.status.clone())
                .or_insert(0) += 1;
        }

        let findings = manager.list_findings_by_project(&p.id)?;
        let raw = findings.iter().filter(|f| f.status == FindingStatus::Raw).count();
        let verified = findings.iter().filter(|f| f.status == FindingStatus::Verified).count();
        let submitted = findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Submitted | FindingStatus::Triaged | FindingStatus::Accepted | FindingStatus::Paid))
            .count();

        overview.total_findings += findings.len();

        for f in &findings {
            *overview
                .findings_by_status
                .entry(f.status.as_str().to_string())
                .or_insert(0) += 1;
        }

        overview.projects.push(ProjectOverview {
            id: p.id.clone(),
            platform: p.platform.clone(),
            root_path: p.root_path.clone(),
            jobs_count: jobs.len(),
            jobs_pending: pending,
            jobs_running: running,
            jobs_done: done,
            jobs_failed: failed,
            last_job_at,
            findings_count: findings.len(),
            raw_count: raw,
            verified_count: verified,
            submitted_count: submitted,
        });
    }

    Ok(serde_json::to_string_pretty(&overview)?)
}

// Helpers

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn format_timestamp(ts_millis: Option<i64>) -> String {
    ts_millis
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_job_id(job: &BugBountyJob) -> String {
    job.kyco_job_id
        .map(|id| format!("#{}", id))
        .unwrap_or_else(|| job.id.clone())
}

fn normalize_root_path(base_abs: &Path, project_dir: &Path) -> String {
    let project_abs = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());

    if let Ok(rel) = project_abs.strip_prefix(base_abs) {
        let rel = rel.to_string_lossy().to_string();
        if !rel.is_empty() {
            return rel;
        }
    }

    project_dir.to_string_lossy().to_string()
}

fn load_project_scope(project_dir: &Path) -> Option<crate::bugbounty::ProjectScope> {
    let scope_path = project_dir.join("scope.md");
    if !scope_path.is_file() {
        return None;
    }

    match parse_scope_file(&scope_path) {
        Ok(scope) => Some(scope),
        Err(err) => {
            eprintln!(
                "Warning: failed to parse scope.md ({}): {}",
                scope_path.display(),
                err
            );
            None
        }
    }
}

fn infer_tool_policy(project_dir: &Path) -> Option<ToolPolicy> {
    let mut policy = ToolPolicy::default();

    // Wrapper convention in this repo: programs/<id>/tools/curl.sh
    if project_dir.join("tools").join("curl.sh").is_file() {
        policy.network_wrapper = Some("./tools/curl.sh".to_string());
        for cmd in ["curl", "wget", "nc", "nmap"] {
            if !policy
                .blocked_commands
                .iter()
                .any(|c| c.eq_ignore_ascii_case(cmd))
            {
                policy.blocked_commands.push(cmd.to_string());
            }
        }
    }

    if project_dir.join("auth").is_dir()
        && !policy
            .protected_paths
            .iter()
            .any(|p| p.eq_ignore_ascii_case("auth/"))
    {
        policy.protected_paths.push("auth/".to_string());
    }

    if policy.network_wrapper.is_none()
        && policy.allowed_commands.is_empty()
        && policy.blocked_commands.is_empty()
        && policy.protected_paths.is_empty()
    {
        None
    } else {
        Some(policy)
    }
}

fn infer_project_metadata(project_dir: &Path) -> Option<ProjectMetadata> {
    let mut meta = ProjectMetadata::default();

    // 1) metadata.json (preferred)
    let json_path = project_dir.join("metadata.json");
    if json_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&json_path) {
            if let Ok(parsed) = serde_json::from_str::<ProjectMetadata>(&content) {
                meta = parsed;
            }
        }
    }

    // 2) metadata.md (simple markdown convention)
    let md_path = project_dir.join("metadata.md");
    if md_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&md_path) {
            if let Some(list) = extract_md_list_section(&content, "Stack") {
                meta.stack = list;
            }
            if let Some(text) = extract_md_text_section(&content, "Auth") {
                if !text.trim().is_empty() {
                    meta.auth_notes = Some(text.trim().to_string());
                }
            }
            if let Some(list) = extract_md_list_section(&content, "Endpoints") {
                meta.endpoints = list;
            }
            if let Some(list) = extract_md_list_section(&content, "Links") {
                meta.links = list;
            }
        }
    }

    // 3) README.md link scraping (best-effort)
    let readme_path = project_dir.join("README.md");
    if readme_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            for url in extract_links_from_text(&content) {
                if !meta.links.iter().any(|l| l == &url) {
                    meta.links.push(url);
                }
            }
        }
    }

    if meta.stack.is_empty()
        && meta
            .auth_notes
            .as_deref()
            .map_or(true, |s| s.trim().is_empty())
        && meta.endpoints.is_empty()
        && meta.links.is_empty()
    {
        None
    } else {
        Some(meta)
    }
}

fn extract_md_text_section(content: &str, heading: &str) -> Option<String> {
    let start = format!("## {}", heading);
    let mut in_section = false;
    let mut buf = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().eq_ignore_ascii_case(&start) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.trim_start().starts_with("## ") {
            break;
        }
        if in_section {
            buf.push(trimmed);
        }
    }

    let out = buf.join("\n").trim().to_string();
    if out.is_empty() { None } else { Some(out) }
}

fn extract_md_list_section(content: &str, heading: &str) -> Option<Vec<String>> {
    let section = extract_md_text_section(content, heading)?;
    let items: Vec<String> = section
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let item = t
                .strip_prefix("- ")
                .or_else(|| t.strip_prefix("* "))
                .map(str::trim)?;
            if item.is_empty() { None } else { Some(item.to_string()) }
        })
        .collect();
    if items.is_empty() { None } else { Some(items) }
}

fn extract_links_from_text(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in content.split_whitespace() {
        let token = raw
            .trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c == '<' || c == '>' || c == ',' || c == '.' || c == ';');
        if token.starts_with("https://") || token.starts_with("http://") {
            out.push(token.to_string());
        }
    }
    out
}
