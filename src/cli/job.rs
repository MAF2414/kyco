//! Job control commands (talk to a running KYCo GUI over the local /ctl API).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::{Job, JobId, JobStatus};

const AUTH_HEADER: &str = "X-KYCO-Token";

#[derive(Debug, serde::Deserialize)]
struct JobsListResponse {
    jobs: Vec<Job>,
}

#[derive(Debug, serde::Deserialize)]
struct JobGetResponse {
    job: Job,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct JobCreateResponse {
    job_ids: Vec<JobId>,
    #[allow(dead_code)]
    group_id: Option<u64>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct JobContinueResponse {
    job_id: JobId,
}

/// Resolve the config path - uses global config (~/.kyco/config.toml) as default,
/// but allows override via --config flag for project-local configs.
fn resolve_config_path(work_dir: &Path, config_override: Option<&PathBuf>) -> PathBuf {
    match config_override {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => work_dir.join(p),
        None => Config::global_config_path(), // Use global config as default
    }
}

fn load_gui_http_settings(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
) -> (u16, Option<String>) {
    // If using default global config, use Config::load() which handles auto-init
    let config = if config_override.is_none() {
        Config::load().ok()
    } else {
        let config_path = resolve_config_path(work_dir, config_override);
        Config::from_file(&config_path).ok()
    };

    let port = config
        .as_ref()
        .map(|c| c.settings.gui.http_port)
        .unwrap_or(9876);
    let token =
        config.and_then(|c| Some(c.settings.gui.http_token).filter(|t| !t.trim().is_empty()));

    (port, token)
}

fn with_auth(mut req: ureq::Request, token: Option<&str>) -> ureq::Request {
    if let Some(token) = token.filter(|t| !t.trim().is_empty()) {
        req = req.set(AUTH_HEADER, token);
    }
    req
}

fn http_get_json(url: &str, token: Option<&str>) -> Result<serde_json::Value> {
    let req = with_auth(ureq::get(url), token);
    let resp = req.call().map_err(|e| match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            anyhow::anyhow!("HTTP {code}: {body}")
        }
        other => anyhow::anyhow!(other),
    })?;

    let body = resp.into_string().context("Failed to read response body")?;
    let json: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse JSON response")?;
    Ok(json)
}

fn http_post_json(
    url: &str,
    token: Option<&str>,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let req = with_auth(ureq::post(url), token).set("Content-Type", "application/json");
    let resp = req
        .send_string(&serde_json::to_string(&payload).context("Failed to serialize request JSON")?)
        .map_err(|e| match e {
            ureq::Error::Status(code, resp) => {
                let body = resp.into_string().unwrap_or_default();
                anyhow::anyhow!("HTTP {code}: {body}")
            }
            other => anyhow::anyhow!(other),
        })?;

    let body = resp.into_string().context("Failed to read response body")?;
    let json: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse JSON response")?;
    Ok(json)
}

pub fn job_list_command(
    work_dir: &Path,
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

    // Apply filters
    let search_lower = search.map(|s| s.to_lowercase());
    let mut jobs: Vec<Job> = parsed
        .jobs
        .into_iter()
        .filter(|job| {
            // Status filter
            if let Some(status) = status_filter {
                let job_status = format!("{}", job.status).to_lowercase();
                if !job_status.contains(&status.to_lowercase()) {
                    return false;
                }
            }
            // Mode filter
            if let Some(mode) = mode_filter {
                if !job.mode.to_lowercase().contains(&mode.to_lowercase()) {
                    return false;
                }
            }
            // Search filter (searches in description, target, and mode)
            if let Some(ref query) = search_lower {
                let desc_match = job
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(query))
                    .unwrap_or(false);
                let target_match = job.target.to_lowercase().contains(query);
                let mode_match = job.mode.to_lowercase().contains(query);
                if !desc_match && !target_match && !mode_match {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by ID descending (newest first)
    jobs.sort_by(|a, b| b.id.cmp(&a.id));

    // Apply limit
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

    // Build filter info string
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
            job.id, job.status, job.mode, job.target
        );
        if let Some(desc) = job.description {
            if !desc.trim().is_empty() {
                // Truncate long descriptions
                let truncated = if desc.len() > 100 {
                    format!("{}...", &desc[..97])
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

pub fn job_get_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    json: bool,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}");
    let value = http_get_json(&url, token.as_deref())?;
    let parsed: JobGetResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs/{id} response")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&parsed.job)?);
        return Ok(());
    }

    let job = parsed.job;
    println!("#{} [{}] {} - {}", job.id, job.status, job.mode, job.target);
    if let Some(desc) = job.description {
        if !desc.trim().is_empty() {
            println!("{}", desc.trim());
        }
    }
    if let Some(err) = job.error_message {
        println!("Error: {}", err);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct JobStartArgs {
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub selected_text: Option<String>,
    pub mode: String,
    pub prompt: Option<String>,
    pub agent: Option<String>,
    pub agents: Vec<String>,
    pub queue: bool,
    pub force_worktree: bool,
    pub json: bool,
}

pub fn job_start_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    args: JobStartArgs,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs");

    let agents = args
        .agents
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let payload = serde_json::json!({
        "file_path": args.file_path,
        "line_start": args.line_start,
        "line_end": args.line_end,
        "selected_text": args.selected_text,
        "mode": args.mode,
        "prompt": args.prompt,
        "agent": args.agent,
        "agents": if agents.is_empty() { None::<Vec<String>> } else { Some(agents) },
        "queue": args.queue,
        "force_worktree": args.force_worktree,
    });

    let value = http_post_json(&url, token.as_deref(), payload)?;
    let parsed: JobCreateResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs response")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&parsed)?);
        return Ok(());
    }

    if parsed.job_ids.len() == 1 {
        println!("Created job #{}", parsed.job_ids[0]);
    } else {
        println!(
            "Created jobs: {}",
            parsed
                .job_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(())
}

pub fn job_queue_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/queue");
    let _ = http_post_json(&url, token.as_deref(), serde_json::json!({}))?;
    println!("Queued job #{}", job_id);
    Ok(())
}

pub fn job_abort_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/abort");
    let _ = http_post_json(&url, token.as_deref(), serde_json::json!({}))?;
    println!("Aborted job #{}", job_id);
    Ok(())
}

pub fn job_delete_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    cleanup_worktree: bool,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/delete");
    let _ = http_post_json(
        &url,
        token.as_deref(),
        serde_json::json!({ "cleanup_worktree": cleanup_worktree }),
    )?;
    println!(
        "Deleted job #{}{}",
        job_id,
        if cleanup_worktree {
            " (worktree cleanup requested)"
        } else {
            ""
        }
    );
    Ok(())
}

pub fn job_continue_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    prompt: String,
    queue: bool,
    json: bool,
) -> Result<()> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        anyhow::bail!("Missing prompt");
    }

    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/continue");
    let payload = serde_json::json!({ "prompt": prompt, "queue": queue });
    let value = http_post_json(&url, token.as_deref(), payload)?;
    let parsed: JobContinueResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs/{id}/continue response")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&parsed)?);
    } else {
        println!("Created continuation job #{}", parsed.job_id);
    }

    Ok(())
}

pub fn job_wait_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    timeout: Option<Duration>,
    poll_interval: Duration,
    json: bool,
) -> Result<()> {
    let deadline = timeout.map(|t| Instant::now() + t);

    loop {
        let job = fetch_job(work_dir, config_override, job_id)?;
        if is_terminal_status(job.status) {
            if json {
                println!("{}", serde_json::to_string_pretty(&job)?);
            } else {
                println!("#{} [{}] {} - {}", job.id, job.status, job.mode, job.target);
            }
            return Ok(());
        }

        if deadline.is_some_and(|d| Instant::now() >= d) {
            anyhow::bail!("Timed out waiting for job #{}", job_id);
        }

        std::thread::sleep(poll_interval);
    }
}

pub fn job_output_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    json: bool,
    summary: bool,
    state: bool,
) -> Result<()> {
    let job = fetch_job(work_dir, config_override, job_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&job)?);
        return Ok(());
    }

    if state {
        let s = job
            .result
            .as_ref()
            .and_then(|r| r.state.as_deref())
            .unwrap_or("");
        println!("{s}");
        return Ok(());
    }

    if summary {
        let s = job
            .result
            .as_ref()
            .and_then(|r| r.summary.as_deref())
            .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()))
            .unwrap_or("");
        println!("{s}");
        return Ok(());
    }

    let out = job
        .full_response
        .as_deref()
        .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()))
        .unwrap_or("");
    println!("{out}");
    Ok(())
}

fn fetch_job(work_dir: &Path, config_override: Option<&PathBuf>, job_id: JobId) -> Result<Job> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}");
    let value = http_get_json(&url, token.as_deref())?;
    let parsed: JobGetResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs/{id} response")?;
    Ok(parsed.job)
}

fn is_terminal_status(status: JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Done | JobStatus::Failed | JobStatus::Rejected | JobStatus::Merged
    )
}
