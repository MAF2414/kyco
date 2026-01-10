//! Job listing command implementation.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::Job;

use super::http::{http_get_json, load_gui_http_settings};
use super::types::JobsListResponse;

pub fn job_list_command(
    work_dir: &std::path::Path,
    config_override: Option<&PathBuf>,
    json: bool,
    status_filter: Option<&str>,
    limit: Option<usize>,
    search: Option<&str>,
    mode_filter: Option<&str>,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs");
    let value = http_get_json(&url, token.as_deref())?;
    let parsed: JobsListResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs response")?;

    let search_lower = search.map(|s| s.to_lowercase());
    let mut jobs: Vec<Job> = parsed
        .jobs
        .into_iter()
        .filter(|job| {
            if let Some(status) = status_filter {
                let job_status = format!("{}", job.status).to_lowercase();
                if !job_status.contains(&status.to_lowercase()) {
                    return false;
                }
            }
            if let Some(mode) = mode_filter {
                if !job.skill.to_lowercase().contains(&mode.to_lowercase()) {
                    return false;
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
                    return false;
                }
            }
            true
        })
        .collect();

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
        println!(
            "  #{} [{}] {} - {}",
            job.id, job.status, job.skill, job.target
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
