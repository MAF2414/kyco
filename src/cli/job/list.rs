//! Job listing command implementation.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::bugbounty::BugBountyManager;
use crate::Job;

use super::http::{http_get_json, load_gui_http_settings};
use super::types::JobsListResponse;

pub fn job_list_command(
    work_dir: &std::path::Path,
    config_override: Option<&PathBuf>,
    json: bool,
    project_filter: Option<&str>,
    finding_filter: Option<&str>,
    status_filter: Option<&str>,
    state_filter: Option<&str>,
    limit: Option<usize>,
    search: Option<&str>,
    mode_filter: Option<&str>,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs");
    let value = http_get_json(&url, token.as_deref())?;
    let parsed: JobsListResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs response")?;

    let (bb_manager, allowed_bb_job_ids): (Option<BugBountyManager>, Option<HashSet<String>>) =
        if let Some(finding_id) = finding_filter {
            let bb = BugBountyManager::new().context("Failed to initialize BugBounty database")?;
            let ids = bb
                .job_findings()
                .list_jobs_for_finding(finding_id)
                .with_context(|| format!("Failed to list jobs for finding {}", finding_id))?;
            (Some(bb), Some(ids.into_iter().collect()))
        } else {
            (None, None)
        };

    let search_lower = search.map(|s| s.to_lowercase());

    let mut jobs: Vec<Job> = Vec::new();
    for job in parsed.jobs.into_iter() {
        if let Some(project_id) = project_filter {
            let matches = job
                .bugbounty_project_id
                .as_deref()
                .is_some_and(|p| p.eq_ignore_ascii_case(project_id));
            if !matches {
                continue;
            }
        }

        if let Some(finding_id) = finding_filter {
            let Some(bb) = bb_manager.as_ref() else {
                anyhow::bail!("BugBounty DB not available (required for --finding {})", finding_id);
            };
            let Some(allowed) = allowed_bb_job_ids.as_ref() else {
                anyhow::bail!("BugBounty DB not available (required for --finding {})", finding_id);
            };

            // Only consider jobs that are already associated with a BugBounty project.
            // (Avoids false matches across GUI sessions where KYCo job IDs may restart.)
            let Some(ref job_project_id) = job.bugbounty_project_id else {
                continue;
            };

            let Some(bb_job) = bb.jobs().get_by_kyco_job_id(job.id)? else {
                continue;
            };
            if bb_job.project_id.as_deref() != Some(job_project_id.as_str()) {
                continue;
            }
            if !allowed.contains(&bb_job.id) {
                continue;
            }
        }

        if let Some(status) = status_filter {
            let job_status = format!("{}", job.status).to_lowercase();
            if !job_status.contains(&status.to_lowercase()) {
                continue;
            }
        }

        if let Some(state) = state_filter {
            let want = state.to_lowercase();
            let have = job
                .result
                .as_ref()
                .and_then(|r| r.state.as_deref())
                .unwrap_or("")
                .to_lowercase();
            if !have.contains(&want) {
                continue;
            }
        }

        if let Some(mode) = mode_filter {
            if !job.skill.to_lowercase().contains(&mode.to_lowercase()) {
                continue;
            }
        }

        if let Some(ref query) = search_lower {
            let desc_match = job
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(query))
                .unwrap_or(false);
            let target_match = job.target.to_lowercase().contains(query);
            let mode_match = job.skill.to_lowercase().contains(query);
            if !desc_match && !target_match && !mode_match {
                continue;
            }
        }

        jobs.push(job);
    }

    // Sort by ID descending (newest first)
    jobs.sort_by(|a, b| b.id.cmp(&a.id));

    if let Some(n) = limit {
        jobs.truncate(n);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    if jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    let mut filters = Vec::new();
    if let Some(s) = status_filter {
        filters.push(format!("status={}", s));
    }
    if let Some(p) = project_filter {
        filters.push(format!("project={}", p));
    }
    if let Some(f) = finding_filter {
        filters.push(format!("finding={}", f));
    }
    if let Some(s) = state_filter {
        filters.push(format!("state={}", s));
    }
    if let Some(m) = mode_filter {
        filters.push(format!("mode={}", m));
    }
    if let Some(q) = search {
        filters.push(format!("search=\"{}\"", q));
    }
    if let Some(n) = limit {
        filters.push(format!("limit={}", n));
    }

    if filters.is_empty() {
        println!("Jobs ({}):\n", jobs.len());
    } else {
        println!("Jobs ({}, {}):\n", jobs.len(), filters.join(", "));
    }

    for job in jobs {
        let project_suffix = job
            .bugbounty_project_id
            .as_deref()
            .map(|p| format!(" (bb:{})", p))
            .unwrap_or_default();
        println!(
            "  #{} [{}] {} - {}{}",
            job.id, job.status, job.skill, job.target, project_suffix
        );
        if let Some(desc) = job.description {
            if !desc.trim().is_empty() {
                let truncated = if desc.chars().count() > 100 {
                    let truncate_at = desc
                        .char_indices()
                        .nth(97)
                        .map(|(i, _)| i)
                        .unwrap_or(desc.len());
                    format!("{}...", &desc[..truncate_at])
                } else {
                    desc
                };
                println!("    {}", truncated.trim());
            }
        }
        if let Some(err) = job.error_message {
            println!("    Error: {}", err);
        }
        println!();
    }

    Ok(())
}
