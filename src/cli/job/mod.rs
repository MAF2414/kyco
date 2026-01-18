//! Job control commands (talk to a running KYCo GUI over the local /ctl API).

mod http;
mod list;
mod types;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{Job, JobId, JobStatus};
use crate::bugbounty::NextContext;

use http::{http_get_json, http_post_json, load_gui_http_settings};
use types::{JobContinueResponse, JobCreateResponse, JobGetResponse};

// Re-export public API
pub use list::job_list_command;
pub use types::JobStartArgs;

pub(crate) fn ctl_create_jobs(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    payload: serde_json::Value,
) -> Result<JobCreateResponse> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs");
    let value = http_post_json(&url, token.as_deref(), payload)?;
    let parsed: JobCreateResponse =
        serde_json::from_value(value).context("Invalid /ctl/jobs response")?;
    Ok(parsed)
}

fn expand_tilde(path: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if path == "~" {
        return Some(home);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Some(home.join(rest));
    }
    #[cfg(windows)]
    if let Some(rest) = path.strip_prefix("~\\") {
        return Some(home.join(rest));
    }
    None
}

fn is_explicit_relative(raw: &str) -> bool {
    raw == "."
        || raw == ".."
        || raw.starts_with("./")
        || raw.starts_with("../")
        || raw.starts_with(".\\")
        || raw.starts_with("..\\")
}

fn resolve_path_candidates(work_dir: &Path, cwd: &Path, raw: &str) -> Vec<PathBuf> {
    let path = expand_tilde(raw).unwrap_or_else(|| PathBuf::from(raw));
    if path.is_absolute() {
        return vec![path];
    }

    // Backwards compatibility: by default resolve relative paths against `work_dir` (the CLI
    // workspace root). For orchestrators that `cd` into a subdir and pass `./file`, treat that as
    // explicitly relative to the process CWD.
    let prefer_cwd = is_explicit_relative(raw);
    let first_base = if prefer_cwd { cwd } else { work_dir };
    let second_base = if prefer_cwd { work_dir } else { cwd };

    let mut candidates = Vec::new();
    candidates.push(first_base.join(&path));
    if second_base != first_base {
        candidates.push(second_base.join(path));
    }
    candidates
}

fn resolve_existing_path(work_dir: &Path, cwd: &Path, raw: &str) -> Result<PathBuf> {
    let candidates = resolve_path_candidates(work_dir, cwd, raw);
    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    anyhow::bail!(
        "Input not found: {raw} (tried: {})",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn looks_like_glob_pattern(raw: &str) -> bool {
    raw.contains('*') || raw.contains('?') || raw.contains('[')
}

fn collect_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == ".git" || name == ".kyco" {
                continue;
            }
            collect_files_recursively(&path, out)?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_dot_slash_relative_to_cwd() -> Result<()> {
        let work_dir = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;

        let file = cwd.path().join("chunk_ac.js");
        std::fs::write(&file, "test")?;

        let resolved = resolve_existing_path(work_dir.path(), cwd.path(), "./chunk_ac.js")?;
        assert_eq!(resolved.canonicalize()?, file.canonicalize()?);
        Ok(())
    }

    #[test]
    fn falls_back_to_cwd_for_bare_relative() -> Result<()> {
        let work_dir = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;

        let file = cwd.path().join("chunk_al.js");
        std::fs::write(&file, "test")?;

        let resolved = resolve_existing_path(work_dir.path(), cwd.path(), "chunk_al.js")?;
        assert_eq!(resolved.canonicalize()?, file.canonicalize()?);
        Ok(())
    }

    #[test]
    fn prefers_work_dir_for_bare_relative_when_both_exist() -> Result<()> {
        let work_dir = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;

        let work_file = work_dir.path().join("same.js");
        let cwd_file = cwd.path().join("same.js");
        std::fs::write(&work_file, "work")?;
        std::fs::write(&cwd_file, "cwd")?;

        let resolved = resolve_existing_path(work_dir.path(), cwd.path(), "same.js")?;
        assert_eq!(resolved.canonicalize()?, work_file.canonicalize()?);

        let resolved = resolve_existing_path(work_dir.path(), cwd.path(), "./same.js")?;
        assert_eq!(resolved.canonicalize()?, cwd_file.canonicalize()?);
        Ok(())
    }

    #[test]
    fn glob_falls_back_to_cwd_when_work_dir_has_no_matches() -> Result<()> {
        let work_dir = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;

        let file = cwd.path().join("a.js");
        std::fs::write(&file, "test")?;

        let inputs = vec!["*.js".to_string()];
        let resolved = expand_input_files_with_cwd(work_dir.path(), cwd.path(), &inputs)?;
        assert_eq!(resolved, vec![file.canonicalize()?]);
        Ok(())
    }

    #[test]
    fn explicit_relative_glob_prefers_cwd_even_if_work_dir_matches() -> Result<()> {
        let work_dir = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;

        let work_file = work_dir.path().join("w.js");
        let cwd_file = cwd.path().join("c.js");
        std::fs::write(&work_file, "work")?;
        std::fs::write(&cwd_file, "cwd")?;

        let inputs = vec!["./*.js".to_string()];
        let resolved = expand_input_files_with_cwd(work_dir.path(), cwd.path(), &inputs)?;
        assert_eq!(resolved, vec![cwd_file.canonicalize()?]);
        Ok(())
    }
}

fn expand_input_files(work_dir: &Path, inputs: &[String]) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| work_dir.to_path_buf());
    expand_input_files_with_cwd(work_dir, &cwd, inputs)
}

fn expand_input_files_with_cwd(work_dir: &Path, cwd: &Path, inputs: &[String]) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut unmatched_globs: Vec<String> = Vec::new();

    for raw in inputs {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        if looks_like_glob_pattern(raw) {
            let mut matched_any = false;
            for pattern_path in resolve_path_candidates(work_dir, cwd, raw) {
                let pattern = pattern_path.to_string_lossy().to_string();
                let mut matched_this_candidate = false;
                for entry in glob::glob(&pattern).with_context(|| format!("Invalid glob: {raw}"))? {
                    let path = match entry {
                        Ok(p) => p,
                        Err(err) => {
                            tracing::warn!("Glob match error for {}: {}", raw, err);
                            continue;
                        }
                    };
                    if path.is_file() {
                        files.push(path);
                        matched_any = true;
                        matched_this_candidate = true;
                    } else if path.is_dir() {
                        collect_files_recursively(&path, &mut files)?;
                        matched_any = true;
                        matched_this_candidate = true;
                    }
                }

                // Fallback only if the preferred base had zero matches.
                if matched_this_candidate {
                    break;
                }
            }
            if !matched_any {
                unmatched_globs.push(raw.to_string());
            }
            continue;
        }

        let resolved = resolve_existing_path(work_dir, cwd, raw)?;
        if resolved.is_file() {
            files.push(resolved);
        } else if resolved.is_dir() {
            collect_files_recursively(&resolved, &mut files)?;
        } else {
            anyhow::bail!("Invalid input path: {}", resolved.display());
        }
    }

    if !unmatched_globs.is_empty() && files.is_empty() {
        anyhow::bail!(
            "No files matched for glob(s): {}",
            unmatched_globs.join(", ")
        );
    }

    // Best-effort de-dupe and stable ordering.
    let mut normalized: Vec<PathBuf> = files
        .into_iter()
        .map(|p| p.canonicalize().unwrap_or(p))
        .collect();
    normalized.sort();
    normalized.dedup();

    Ok(normalized)
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
    println!("#{} [{}] {} - {}", job.id, job.status, job.skill, job.target);
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
    let input = args
        .input
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let uses_input = !input.is_empty();

    if uses_input && args.file_path.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        anyhow::bail!("Use either --file or --input (not both)");
    }
    if args.batch && !uses_input {
        anyhow::bail!("--batch requires --input");
    }

    // Validate: need either --file/--input or --prompt (or both)
    let file_path_raw = args.file_path.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let prompt_provided = args.prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();

    if file_path_raw.is_none() && !uses_input && !prompt_provided {
        anyhow::bail!("Either --file/--input or --prompt (or both) must be provided");
    }

    if let Some(start) = args.line_start {
        if start == 0 {
            anyhow::bail!("--line-start must be >= 1");
        }
    }
    if let Some(end) = args.line_end {
        if end == 0 {
            anyhow::bail!("--line-end must be >= 1");
        }
    }
    if let (Some(start), Some(end)) = (args.line_start, args.line_end) {
        if end < start {
            anyhow::bail!("--line-end must be >= --line-start");
        }
    }

    let input_files: Vec<PathBuf> = if uses_input {
        expand_input_files(work_dir, &input)?
    } else {
        Vec::new()
    };

    if !args.batch && input_files.len() > 1 {
        anyhow::bail!(
            "--input resolved to {} files. Use --batch to create one job per input.",
            input_files.len()
        );
    }

    // Resolve file path if provided (single file mode)
    let cwd = std::env::current_dir().unwrap_or_else(|_| work_dir.to_path_buf());
    let single_file_path: Option<String> = if let Some(raw) = file_path_raw {
        let resolved_path = resolve_existing_path(work_dir, &cwd, raw)?;
        if !resolved_path.is_file() {
            anyhow::bail!("Path is not a file: {}", resolved_path.display());
        }
        let resolved_path = resolved_path.canonicalize().unwrap_or(resolved_path);
        Some(resolved_path.display().to_string())
    } else if !input_files.is_empty() {
        Some(input_files[0].display().to_string())
    } else {
        None
    };

    let agents = args
        .agents
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let bugbounty_finding_ids = args
        .bugbounty_finding_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let bugbounty_finding_ids =
        if bugbounty_finding_ids.is_empty() { None } else { Some(bugbounty_finding_ids) };

    let mode = args.mode.clone();
    let prompt = args.prompt.clone();
    let selected_text = args.selected_text.clone();
    let bugbounty_project_id = args.bugbounty_project_id.clone();
    let agent = args.agent.clone();

    let mut batch_results: Vec<(Option<String>, JobCreateResponse)> = Vec::new();

    if args.batch {
        for path in &input_files {
            let payload = serde_json::json!({
                "file_path": path.display().to_string(),
                "line_start": args.line_start,
                "line_end": args.line_end,
                "selected_text": selected_text.clone(),
                "mode": mode.clone(),
                "prompt": prompt.clone(),
                "bugbounty_project_id": bugbounty_project_id.clone(),
                "bugbounty_finding_ids": bugbounty_finding_ids.clone(),
                "agent": agent.clone(),
                "agents": if agents.is_empty() { None::<Vec<String>> } else { Some(agents.clone()) },
                "queue": args.queue,
                "force_worktree": args.force_worktree,
            });
            let parsed = ctl_create_jobs(work_dir, config_override, payload)?;
            batch_results.push((Some(path.display().to_string()), parsed));
        }
    } else {
        let payload = serde_json::json!({
            "file_path": single_file_path,
            "line_start": args.line_start,
            "line_end": args.line_end,
            "selected_text": selected_text,
            "mode": mode,
            "prompt": prompt,
            "bugbounty_project_id": bugbounty_project_id,
            "bugbounty_finding_ids": bugbounty_finding_ids,
            "agent": agent,
            "agents": if agents.is_empty() { None::<Vec<String>> } else { Some(agents) },
            "queue": args.queue,
            "force_worktree": args.force_worktree,
        });
        let parsed = ctl_create_jobs(work_dir, config_override, payload)?;
        batch_results.push((single_file_path.clone(), parsed));
    }

    if args.json {
        if batch_results.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&batch_results[0].1)?);
            return Ok(());
        }

        let flattened = batch_results
            .iter()
            .flat_map(|(_, r)| r.job_ids.iter().copied())
            .collect::<Vec<_>>();

        let output = serde_json::json!({
            "jobs": batch_results.iter().map(|(input, r)| serde_json::json!({
                "input": input,
                "job_ids": r.job_ids,
                "group_id": r.group_id,
            })).collect::<Vec<_>>(),
            "job_ids": flattened,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if batch_results.len() == 1 {
        let parsed = &batch_results[0].1;
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
        return Ok(());
    }

    println!(
        "Created {} job groups across {} inputs",
        batch_results.len(),
        input_files.len()
    );
    for (input, parsed) in batch_results {
        let ids = parsed
            .job_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(input) = input {
            println!("  {} → {}", input, ids);
        } else {
            println!("  {}", ids);
        }
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

pub fn job_kill_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/kill");
    let _ = http_post_json(&url, token.as_deref(), serde_json::json!({}))?;
    println!("Killed job #{}", job_id);
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
                println!("#{} [{}] {} - {}", job.id, job.status, job.skill, job.target);
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
    next_context: bool,
    findings: bool,
    flow: bool,
    artifacts: bool,
    summary: bool,
    state: bool,
) -> Result<()> {
    let job = fetch_job(work_dir, config_override, job_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&job)?);
        return Ok(());
    }

    if next_context || findings || flow || artifacts {
        let ctx = job
            .result
            .as_ref()
            .and_then(|r| r.next_context.clone())
            .and_then(|v| NextContext::from_value(v).ok())
            .or_else(|| {
                let text = job
                    .full_response
                    .as_deref()
                    .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()))
                    .unwrap_or("");
                NextContext::extract_from_text(text)
            })
            .ok_or_else(|| anyhow::anyhow!("No next_context found in job output"))?;

        if next_context {
            println!("{}", serde_json::to_string_pretty(&ctx)?);
            return Ok(());
        }

        let selected = [findings, flow, artifacts].into_iter().filter(|v| *v).count();
        if selected > 1 {
            let mut output = serde_json::Map::new();
            if findings {
                output.insert("findings".to_string(), serde_json::to_value(&ctx.findings)?);
            }
            if flow {
                output.insert("flow_edges".to_string(), serde_json::to_value(&ctx.flow_edges)?);
            }
            if artifacts {
                output.insert("artifacts".to_string(), serde_json::to_value(&ctx.artifacts)?);
            }
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        if findings {
            println!("{}", serde_json::to_string_pretty(&ctx.findings)?);
            return Ok(());
        }

        if flow {
            println!("{}", serde_json::to_string_pretty(&ctx.flow_edges)?);
            return Ok(());
        }

        if artifacts {
            println!("{}", serde_json::to_string_pretty(&ctx.artifacts)?);
            return Ok(());
        }
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

pub fn job_restart_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    job_id: JobId,
) -> Result<()> {
    let (port, token) = load_gui_http_settings(work_dir, config_override);
    let url = format!("http://127.0.0.1:{port}/ctl/jobs/{job_id}/restart");
    let value = http_post_json(&url, token.as_deref(), serde_json::json!({}))?;

    let status = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    if status == "ok" {
        let new_job_id = value
            .get("new_job_id")
            .and_then(|id| id.as_u64())
            .unwrap_or(0);
        println!("Restarted job #{} as #{}", job_id, new_job_id);
        Ok(())
    } else {
        let error = value
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown_error");
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Restart failed");
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
