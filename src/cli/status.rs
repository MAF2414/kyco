//! Status command implementation

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::{Job, JobStatus};

const AUTH_HEADER: &str = "X-KYCO-Token";

#[derive(Debug, serde::Deserialize)]
struct JobsListResponse {
    jobs: Vec<Job>,
}

/// Show the status of all jobs
pub async fn status_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    filter: Option<String>,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs");

    let mut req = ureq::get(&url);
    if let Some(token) = token.as_deref() {
        req = req.set(AUTH_HEADER, token);
    }

    let resp = req.call().map_err(|e| match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            anyhow::anyhow!("HTTP {code}: {body}")
        }
        other => anyhow::anyhow!(other),
    })?;

    let body = resp.into_string()?;
    let parsed: JobsListResponse = serde_json::from_str(&body)?;

    let mut jobs = parsed.jobs;
    if let Some(status_filter) = filter {
        let target_status = match status_filter.to_lowercase().as_str() {
            "pending" => Some(JobStatus::Pending),
            "queued" => Some(JobStatus::Queued),
            "blocked" => Some(JobStatus::Blocked),
            "running" => Some(JobStatus::Running),
            "done" => Some(JobStatus::Done),
            "failed" => Some(JobStatus::Failed),
            "rejected" => Some(JobStatus::Rejected),
            "merged" => Some(JobStatus::Merged),
            _ => {
                eprintln!("Unknown status: {}", status_filter);
                return Ok(());
            }
        };
        jobs.retain(|j| Some(j.status) == target_status);
    }

    if jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    println!("Jobs ({}):\n", jobs.len());
    for job in jobs {
        println!(
            "  #{} [{}] {} - {}",
            job.id, job.status, job.mode, job.target
        );

        if let Some(desc) = job.description.as_deref().filter(|d| !d.trim().is_empty()) {
            println!("    {}", desc.trim());
        }

        if let Some(err) = job.error_message.as_deref() {
            println!("    Error: {}", err);
        }

        println!();
    }

    Ok(())
}

fn resolve_config_path(work_dir: &Path, config_override: Option<&PathBuf>) -> PathBuf {
    match config_override {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => work_dir.join(p),
        None => work_dir.join(".kyco").join("config.toml"),
    }
}

fn load_gui_http_settings(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
) -> (u16, Option<String>) {
    let config_path = resolve_config_path(work_dir, config_override);
    let config = Config::from_file(&config_path).ok();

    let port = config
        .as_ref()
        .map(|c| c.settings.gui.http_port)
        .unwrap_or(9876);
    let token =
        config.and_then(|c| Some(c.settings.gui.http_token).filter(|t| !t.trim().is_empty()));

    (port, token)
}
