//! Job control commands (talk to a running KYCo GUI over the local /ctl API).

mod http;
mod list;
mod types;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{Job, JobId, JobStatus};

use http::{http_get_json, http_post_json, load_gui_http_settings};
use types::{JobContinueResponse, JobCreateResponse, JobGetResponse};

// Re-export public API
pub use list::job_list_command;
pub use types::JobStartArgs;

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
    println!("Abort requested for job #{}", job_id);
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

pub fn job_merge_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    message: Option<String>,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/merge");
    let payload = match message {
        Some(msg) => serde_json::json!({ "message": msg }),
        None => serde_json::json!({}),
    };
    let value = http_post_json(&url, token.as_deref(), payload)?;

    let status = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    if status == "ok" {
        let default_msg = format!("Merged job #{}", job_id);
        let msg = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(&default_msg);
        println!("{}", msg);
        Ok(())
    } else {
        let error = value
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown_error");
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Merge failed");
        anyhow::bail!("{}: {}", error, message)
    }
}

pub fn job_reject_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/reject");
    let value = http_post_json(&url, token.as_deref(), serde_json::json!({}))?;

    let status = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    if status == "ok" {
        let default_msg = format!("Rejected job #{}", job_id);
        let msg = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(&default_msg);
        println!("{}", msg);
        Ok(())
    } else {
        let error = value
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown_error");
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Reject failed");
        anyhow::bail!("{}: {}", error, message)
    }
}

pub fn job_diff_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
    json: bool,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/diff");
    let value = http_get_json(&url, token.as_deref())?;

    if let Some(error) = value.get("error").and_then(|e| e.as_str()) {
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Diff failed");
        anyhow::bail!("{}: {}", error, message);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    // Print human-readable output
    if let Some(changed_files) = value.get("changed_files").and_then(|f| f.as_array()) {
        if !changed_files.is_empty() {
            println!("Changed files ({}):", changed_files.len());
            for file in changed_files {
                if let Some(f) = file.as_str() {
                    println!("  {}", f);
                }
            }
            println!();
        }
    }

    if let Some(diff) = value.get("diff").and_then(|d| d.as_str()) {
        if diff.is_empty() {
            println!("No changes");
        } else {
            println!("{}", diff);
        }
    }

    Ok(())
}
