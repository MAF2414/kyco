//! Chain job execution logic

mod worktree;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::agent::{AgentRegistry, ChainProgressEvent, ChainRunner, ChainStepResult};
use crate::bugbounty::{BugBountyJob, BugBountyManager, ContextInjector, NextContext};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{ChainStepSummary, Job, JobResult, JobStatus, LogEvent};

use super::ExecutorEvent;
use super::JobLockGuard;
use super::log_forwarder::spawn_log_forwarder;
use super::worktree_paths::remap_job_paths_to_worktree;
use worktree::setup_chain_worktree;

fn load_active_bugbounty_project() -> Option<String> {
    let path = dirs::home_dir()?.join(".kyco").join("active_project");
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_bugbounty_security_skill(skill: &str) -> bool {
    matches!(
        skill,
        "authz-bypass-hunter"
            | "injection-hunter"
            | "secrets-hunter"
            | "crypto-audit"
            | "jwt-attack-surface"
            | "dos-resource-exhaustion"
            | "go-security-audit"
            | "flow-trace"
    )
}

fn cleaned_finding_ids(finding_ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = finding_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn link_requested_findings(
    bb: &BugBountyManager,
    project_id: &str,
    bb_job_id: &str,
    finding_ids: &[String],
    event_tx: &Sender<ExecutorEvent>,
) {
    let ids = cleaned_finding_ids(finding_ids);
    for fid in ids {
        match bb.get_finding(&fid) {
            Ok(Some(f)) => {
                if f.project_id != project_id {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "BugBounty job link skipped: finding {} belongs to project {}, not {}",
                        fid, f.project_id, project_id
                    ))));
                    continue;
                }
                if let Err(err) = bb.job_findings().link(bb_job_id, &fid, "related") {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "BugBounty job link skipped for finding {}: {}",
                        fid, err
                    ))));
                }
            }
            Ok(None) => {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                    "BugBounty job link skipped: finding {} not found",
                    fid
                ))));
            }
            Err(err) => {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                    "BugBounty job link skipped for finding {}: {}",
                    fid, err
                ))));
            }
        }
    }
}

/// Convert a ChainStepResult to a ChainStepSummary (clones string fields)
fn step_result_to_summary(step_result: &ChainStepResult) -> ChainStepSummary {
    ChainStepSummary {
        step_index: step_result.step_index,
        skill: step_result.skill.to_string(),
        skipped: step_result.skipped,
        success: step_result
            .agent_result
            .as_ref()
            .map(|ar| ar.success)
            .unwrap_or(false),
        title: step_result
            .job_result
            .as_ref()
            .and_then(|jr| jr.title.clone()),
        summary: step_result
            .job_result
            .as_ref()
            .and_then(|jr| jr.summary.clone()),
        full_response: step_result.full_response.clone(),
        error: step_result
            .agent_result
            .as_ref()
            .and_then(|ar| ar.error.clone()),
        files_changed: step_result
            .agent_result
            .as_ref()
            .map(|ar| ar.files_changed)
            .unwrap_or(0),
    }
}

/// Run a job that is actually a chain of modes
pub async fn run_chain_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    mut job: Job,
) {
    let job_id = job.id;
    let chain_name = job.skill.clone();
    let _job_locks = JobLockGuard::new(Arc::clone(job_manager), job_id);

    // Validate job inputs before marking as running.
    if job.source_line == 0 {
        let error = "Invalid job input: source_line must be >= 1".to_string();
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(&error)));
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.fail(&error);
            }
            manager.touch();
        }
        let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
        return;
    }

    let workspace_root = job.workspace_path.as_ref().unwrap_or(work_dir);
    let resolved_source_file = if job.source_file.is_absolute() {
        job.source_file.clone()
    } else {
        workspace_root.join(&job.source_file)
    };

    // Check if this is a prompt-only job (source_file equals workspace root)
    // Prompt-only jobs have no specific source file - they use the workspace as a placeholder
    let is_prompt_only_job =
        resolved_source_file == *workspace_root || job.source_file.to_string_lossy() == "prompt";

    // BugBounty project resolution (prompt-only fallback to active project, else infer from file).
    let mut bugbounty_project_id = job.bugbounty_project_id.clone();
    if is_prompt_only_job && bugbounty_project_id.is_none() {
        if let Some(active_project_id) = load_active_bugbounty_project() {
            bugbounty_project_id = Some(active_project_id.clone());
            job.bugbounty_project_id = Some(active_project_id.clone());
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.bugbounty_project_id = Some(active_project_id);
                }
                manager.touch();
            }
        }
    }
    if bugbounty_project_id.is_none() && !is_prompt_only_job {
        if let Ok(bb) = BugBountyManager::new() {
            if let Ok(Some((project, _))) = bb.infer_project_for_path(work_dir, &resolved_source_file) {
                bugbounty_project_id = Some(project.id.clone());
                job.bugbounty_project_id = Some(project.id.clone());
                if let Ok(mut manager) = job_manager.lock() {
                    if let Some(j) = manager.get_mut(job_id) {
                        j.bugbounty_project_id = Some(project.id.clone());
                    }
                    manager.touch();
                }
            }
        }
    }

    // Only validate source file existence if it's not a prompt-only job
    if !is_prompt_only_job {
        if !resolved_source_file.exists() {
            let error = format!("Source file not found: {}", resolved_source_file.display());
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(&error)));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(&error);
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
        if !resolved_source_file.is_file() {
            let error = format!(
                "Invalid job input: source file is not a file: {}",
                resolved_source_file.display()
            );
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(&error)));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(&error);
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    }
    if !job.source_file.is_absolute() {
        job.source_file = resolved_source_file;
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.source_file = job.source_file.clone();
            }
            manager.touch();
        }
    }

    let chain = match config.get_chain(&chain_name) {
        Some(c) => c.clone(),
        None => {
            let error = format!("Chain '{}' not found", chain_name);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(&error)));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(&error);
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    {
        let Ok(mut manager) = job_manager.lock() else {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Chain job #{} failed: lock poisoned",
                job_id
            ))));
            return;
        };
        manager.set_status(job_id, JobStatus::Running);
        if let Some(j) = manager.get_mut(job_id) {
            j.chain_name = Some(chain_name.clone());
            j.chain_total_steps = Some(chain.steps.len());
            j.chain_current_step = Some(0);
        }
    }

    let _ = event_tx.send(ExecutorEvent::JobStarted(job_id));
    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
        "Starting chain '{}' with {} steps for job #{}",
        chain_name,
        chain.steps.len(),
        job_id
    ))));

    // Multi-agent jobs always require worktrees for isolation
    let is_multi_agent_job = job.group_id.is_some();
    let should_use_worktree =
        config.settings.use_worktree || is_multi_agent_job || job.force_worktree;

    let job_work_dir = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| work_dir.clone());

    // Create GitManager for the job's workspace (may be different from global work_dir)
    let job_git_manager =
        if job.workspace_path.is_some() && job.workspace_path.as_ref() != Some(work_dir) {
            GitManager::new(&job_work_dir).ok()
        } else {
            None
        };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    // Reuse existing worktree when present (e.g., session continuation)
    let (worktree_path, _is_isolated) =
        if let Some(existing_worktree) = job.git_worktree_path.as_ref().filter(|p| p.exists()) {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                "Reusing worktree: {}",
                existing_worktree.display()
            ))));
            (existing_worktree.clone(), true)
        } else if should_use_worktree {
            match setup_chain_worktree(
                effective_git_manager,
                job_id,
                is_multi_agent_job,
                job.force_worktree,
                &job_work_dir,
                event_tx,
                job_manager,
                &mut job,
            ) {
                Some(result) => result,
                None => return,
            }
        } else {
            (job_work_dir.clone(), false)
        };

    if _is_isolated {
        let remap = remap_job_paths_to_worktree(&mut job, &job_work_dir, &worktree_path);
        if remap.copied_source_file {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(
                "Copied source file into worktree to preserve isolation",
            )));
        }
        if remap.remapped {
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.source_file = job.source_file.clone();
                    j.scope = job.scope.clone();
                    j.target.clone_from(&job.target);
                }
                manager.touch();
            }
        }
    }

    // BugBounty context injection + job persistence for chain runs (best-effort).
    let mut bugbounty_job_id: Option<String> = None;
    if let Some(ref project_id) = bugbounty_project_id {
        if let Ok(bb) = BugBountyManager::new() {
            // Inject known findings/scope/tool-policy/output schema into the chain job context.
            match ContextInjector::new(bb.clone()).for_project(project_id) {
                Ok(mut injected) => {
                    if !job.bugbounty_finding_ids.is_empty() {
                        let focus = cleaned_finding_ids(&job.bugbounty_finding_ids)
                            .into_iter()
                            .filter_map(|fid| {
                                bb.get_finding(&fid).ok().flatten().map(|f| {
                                    format!(
                                        "- **{}** [{}] {} ({})",
                                        f.id,
                                        f.severity.map(|s| s.as_str()).unwrap_or("-"),
                                        f.title,
                                        f.status.as_str()
                                    )
                                })
                            })
                            .collect::<Vec<_>>();
                        if !focus.is_empty() {
                            injected.focus_findings = Some(focus.join("\n"));
                        }
                    }

                    let injection = injected.to_system_prompt();
                    if !injection.trim().is_empty() {
                        let combined = match job.ide_context.take() {
                            Some(existing) if !existing.trim().is_empty() => {
                                format!("{existing}\n\n---\n\n{injection}")
                            }
                            _ => injection,
                        };
                        job.ide_context = Some(combined.clone());
                        if let Ok(mut manager) = job_manager.lock() {
                            if let Some(j) = manager.get_mut(job_id) {
                                j.ide_context = Some(combined);
                            }
                            manager.touch();
                        }
                    }
                }
                Err(err) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "BugBounty context injection skipped: {}",
                        err
                    ))));
                }
            }

            // Persist one BugBounty job record for the whole chain (links findings/artifacts).
            let bb_job_id = uuid::Uuid::new_v4().to_string();
            let mut bb_job = BugBountyJob::new(&bb_job_id)
                .with_project_id(project_id.clone())
                .with_kyco_job_id(job_id)
                .with_mode(chain_name.clone())
                .mark_started();
            if let Some(ref prompt) = job.description {
                if !prompt.trim().is_empty() {
                    bb_job = bb_job.with_prompt(prompt.trim().to_string());
                }
            }

            if let Err(err) = bb.jobs().upsert(&bb_job) {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                    "BugBounty job persistence skipped: {}",
                    err
                ))));
            } else {
                link_requested_findings(
                    &bb,
                    project_id,
                    &bb_job_id,
                    &job.bugbounty_finding_ids,
                    event_tx,
                );
                bugbounty_job_id = Some(bb_job_id);
            }
        }
    }

    let (log_tx, log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);
    let log_forwarder =
        spawn_log_forwarder(log_rx, event_tx.clone(), Arc::clone(job_manager), job_id);

    let chain_runner = ChainRunner::new(config, agent_registry, &worktree_path);
    let (progress_tx, progress_rx) = std::sync::mpsc::channel::<ChainProgressEvent>();
    let event_tx_progress = event_tx.clone();
    let job_manager_progress = Arc::clone(job_manager);
    let progress_job_id = job_id;
    let total_steps_for_progress = chain.steps.len();
    let progress_forwarder = tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv() {
            if let Ok(mut manager) = job_manager_progress.lock() {
                if let Some(j) = manager.get_mut(progress_job_id) {
                    if progress.is_starting {
                        j.chain_current_step = Some(progress.step_index);
                    } else {
                        j.chain_current_step = Some(progress.step_index + 1);
                        if let Some(step_result) = &progress.step_result {
                            let summary = step_result_to_summary(step_result);
                            let state = step_result
                                .job_result
                                .as_ref()
                                .and_then(|jr| jr.state.clone());
                            // Clone skill before potentially cloning summary
                            let mode = summary.skill.clone();
                            // Only clone summary if we need it for history
                            let step_summary =
                                if j.chain_step_history.len() <= step_result.step_index {
                                    let clone = summary.clone();
                                    j.chain_step_history.push(summary);
                                    clone
                                } else {
                                    summary
                                };
                            let _ = event_tx_progress.send(ExecutorEvent::ChainStepCompleted {
                                job_id: progress_job_id,
                                step_index: step_result.step_index,
                                total_steps: total_steps_for_progress,
                                mode,
                                state,
                                step_summary,
                            });
                        }
                    }
                }
            }
        }
    });

    let chain_result = chain_runner
        .run_chain(&chain_name, &chain, &job, log_tx, Some(progress_tx))
        .await;

    // Best-effort BugBounty ingestion: parse next_context from each executed step.
    let mut bugbounty_contract_error: Option<String> = None;
    let mut aggregated_ctx = NextContext::default();
    let mut aggregated_value: Option<serde_json::Value> = None;

    if let (Some(project_id), Some(bb_job_id)) =
        (bugbounty_project_id.as_deref(), bugbounty_job_id.as_deref())
    {
        if let Ok(bb) = BugBountyManager::new() {
            for step in &chain_result.step_results {
                if step.skipped {
                    continue;
                }

                let Some(ref output) = step.full_response else {
                    continue;
                };

                let mut ctx: Option<NextContext> = None;
                let mut next_context_value: Option<serde_json::Value> = None;
                let mut result_state: Option<String> = None;

                if let Some(job_result) = JobResult::parse(output) {
                    result_state = job_result.state.clone();
                    if let Some(ref value) = job_result.next_context {
                        next_context_value = Some(value.clone());
                    }
                    if let Some(value) = job_result.next_context {
                        if let Ok(parsed) = NextContext::from_value(value) {
                            ctx = Some(parsed);
                        }
                    }
                }

                if ctx.is_none() {
                    ctx = NextContext::extract_from_text(output);
                }

                // Check contract but don't fail - structured output is optional
                let check_contract =
                    is_bugbounty_security_skill(step.skill.as_ref()) || !job.bugbounty_finding_ids.is_empty();
                if check_contract {
                    match ctx.as_ref() {
                        Some(parsed) => {
                            if let Err(err) = parsed.validate_security_audit() {
                                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                    "BugBounty output in step '{}': {}",
                                    step.skill, err
                                ))));
                                // Don't fail - just log warning
                            }
                        }
                        None => {
                            // No structured output - that's okay for now
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty: no structured output in step '{}'",
                                step.skill
                            ))));
                        }
                    }
                }

                if let Some(ctx) = ctx {
                    if !ctx.is_empty() {
                        if let Err(err) = bb.process_next_context(project_id, &ctx, Some(bb_job_id)) {
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty ingestion skipped for step '{}': {}",
                                step.skill, err
                            ))));
                        } else {
                            aggregated_ctx.findings.extend(ctx.findings);
                            aggregated_ctx.flow_edges.extend(ctx.flow_edges);
                            aggregated_ctx.artifacts.extend(ctx.artifacts);
                        }
                    }
                }

                // Keep last non-empty next_context for debugging/auditing.
                if let Some(value) = next_context_value {
                    aggregated_value = Some(value);
                } else if result_state.is_some() {
                    // silence unused warning, result_state may be used later if needed.
                }
            }

            if !aggregated_ctx.is_empty() {
                aggregated_value = serde_json::to_value(&aggregated_ctx).ok();
            }

            let completed_at = chrono::Utc::now().timestamp_millis();
            let ok = chain_result.success && bugbounty_contract_error.is_none();
            let status = if ok { "done" } else { "failed" };
            let _ = bb.jobs().mark_completed(
                bb_job_id,
                status,
                completed_at,
                chain_result.final_state.as_deref(),
                aggregated_value.as_ref(),
            );
        }
    }

    tokio::time::sleep(Duration::from_millis(50)).await;
    progress_forwarder.abort();
    let _ = progress_forwarder.await;

    let chain_ok = chain_result.success && bugbounty_contract_error.is_none();
    let total_steps = chain_result.step_results.len();
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            let mut combined_details = Vec::new();
            let mut total_files_changed = 0;
            let mut step_history = Vec::new();

            for step_result in &chain_result.step_results {
                let summary = step_result_to_summary(step_result);

                if step_result.skipped {
                    combined_details.push(format!("[{}] skipped", summary.skill));
                } else {
                    if let Some(title) = &summary.title {
                        combined_details.push(format!("[{}] {}", summary.skill, title));
                    }
                    total_files_changed += summary.files_changed;
                }

                step_history.push(summary);
            }

            j.chain_step_history = step_history;
            j.chain_current_step = Some(total_steps);

            j.result = Some(JobResult {
                title: Some(format!("Chain '{}' completed", chain_name)),
                commit_subject: None,
                commit_body: None,
                details: Some(combined_details.join("\n")),
                status: Some(
                    if chain_ok {
                        "success"
                    } else {
                        "partial"
                    }
                    .to_string(),
                ),
                summary: Some(chain_result.accumulated_summaries.join("\n\n")),
                state: chain_result.final_state.clone(),
                next_context: aggregated_value.clone(),
                raw_text: None,
            });

            j.set_file_stats(total_files_changed, 0, 0);

            if chain_ok {
                j.set_status(JobStatus::Done);
                let _ = event_tx.send(ExecutorEvent::JobCompleted(job_id));
            } else {
                j.set_status(JobStatus::Failed);
                let fail_message = bugbounty_contract_error
                    .clone()
                    .unwrap_or_else(|| "Chain execution failed".to_string());
                j.error_message = Some(fail_message.clone());
                let _ = event_tx.send(ExecutorEvent::JobFailed(
                    job_id,
                    fail_message,
                ));
            }
        }
        manager.touch();
    }

    let _ = event_tx.send(ExecutorEvent::ChainCompleted {
        job_id,
        chain_name,
        steps_executed: chain_result
            .step_results
            .iter()
            .filter(|r| !r.skipped)
            .count(),
        success: chain_result.success,
    });

    let _ = log_forwarder.await;
}
