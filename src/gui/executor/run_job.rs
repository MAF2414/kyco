//! Single job execution logic

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::agent::AgentRegistry;
use crate::bugbounty::{BugBountyJob, BugBountyManager, ContextInjector};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{Job, JobStatus, LogEvent};

use super::ExecutorEvent;
use super::JobLockGuard;
use super::chain::run_chain_job;
use super::git_utils::calculate_git_numstat_async;
use super::log_forwarder::spawn_log_forwarder;
use super::worktree_paths::remap_job_paths_to_worktree;
use super::worktree_setup::setup_worktree;

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

/// Run a single job (non-chain)
pub async fn run_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    mut job: Job,
) {
    let job_id = job.id;
    let _job_locks = JobLockGuard::new(Arc::clone(job_manager), job_id);

    if config.is_chain(&job.skill) {
        run_chain_job(
            work_dir,
            config,
            job_manager,
            agent_registry,
            git_manager,
            event_tx,
            job,
        )
        .await;
        return;
    }

    // Validate job inputs before marking as running.
    if job.source_line == 0 {
        let error = "Invalid job input: source_line must be >= 1".to_string();
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.fail(error.clone());
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

    // Only validate source file existence if it's not a prompt-only job
    if !is_prompt_only_job {
        if !resolved_source_file.exists() {
            let error = format!("Source file not found: {}", resolved_source_file.display());
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
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
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
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

    // BugBounty (best-effort): infer project and inject context into the prompt.
    // Non-fatal by design: regular KYCo jobs should still work without BugBounty data.
    // BugBounty project roots (`project.root_path`) are stored relative to the GUI work_dir.
    // Do NOT resolve them relative to `job.workspace_path` (Kanban quick-actions may set that to the
    // project root already), otherwise we can end up with duplicated paths.
    let bb_work_dir = work_dir;
    let mut bugbounty_project_id: Option<String> = job.bugbounty_project_id.clone();
    let mut bugbounty_job_id: Option<String> = None;

    fn cleaned_finding_ids(ids: &[String]) -> Vec<String> {
        ids.iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn format_focus_findings(
        bb: &BugBountyManager,
        project_id: &str,
        finding_ids: &[String],
    ) -> Option<String> {
        let ids = cleaned_finding_ids(finding_ids);
        if ids.is_empty() {
            return None;
        }

        let mut lines = Vec::new();
        lines.push("These findings are explicitly linked to this job (verification/triage targets).".to_string());
        lines.push("If you update an existing finding, include its `id` in `next_context.findings[]`.".to_string());
        lines.push(String::new());

        for fid in ids {
            match bb.get_finding(&fid).ok().flatten() {
                Some(f) if f.project_id == project_id => {
                    let sev = f
                        .severity
                        .map(|s| s.as_str().to_uppercase())
                        .unwrap_or_else(|| "-".to_string());
                    lines.push(format!(
                        "- **{}** [{}] {} ({})",
                        f.id,
                        sev,
                        f.title,
                        f.status.as_str()
                    ));
                    if let Some(ref scenario) = f.attack_scenario {
                        lines.push(format!("  - Attack: {}", scenario));
                    }
                    if !f.affected_assets.is_empty() {
                        lines.push(format!("  - Assets: {}", f.affected_assets.join(", ")));
                    }
                    if let Some(ref taint) = f.taint_path {
                        lines.push(format!("  - Taint: {}", taint));
                    }
                }
                Some(f) => {
                    lines.push(format!(
                        "- **{}** (belongs to project `{}`; current job project is `{}`)",
                        fid, f.project_id, project_id
                    ));
                }
                None => {
                    lines.push(format!("- **{}** (not found)", fid));
                }
            }
        }

        Some(lines.join("\n"))
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

    if is_prompt_only_job {
        // Prompt-only jobs cannot infer the project from a file path.
        // Fall back to the globally persisted selection (used by the Kanban GUI).
        if bugbounty_project_id.is_none() {
            if let Some(active_project_id) = load_active_bugbounty_project() {
                bugbounty_project_id = Some(active_project_id.clone());
                job.bugbounty_project_id = Some(active_project_id.clone());
                if let Ok(mut manager) = job_manager.lock() {
                    if let Some(j) = manager.get_mut(job_id) {
                        j.bugbounty_project_id = Some(active_project_id.clone());
                    }
                    manager.touch();
                }
            }
        }

        if let Some(project_id) = bugbounty_project_id.clone() {
            if let Ok(bb) = BugBountyManager::new() {
                if bb.get_project(&project_id).ok().flatten().is_none() {
                    // If the explicit/active project id isn't registered, skip BugBounty integration.
                    bugbounty_project_id = None;
                } else {
                    match ContextInjector::new(bb.clone()).for_project(&project_id) {
                        Ok(mut injected) => {
                            if let Some(focus) =
                                format_focus_findings(&bb, &project_id, &job.bugbounty_finding_ids)
                            {
                                injected.focus_findings = Some(focus);
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

                // Persist a BugBounty job record (used for linking findings/artifacts).
                let bb_job_id = uuid::Uuid::new_v4().to_string();
                let mut bb_job = BugBountyJob::new(&bb_job_id)
                    .with_project_id(project_id.clone())
                    .with_kyco_job_id(job_id)
                    .with_mode(job.skill.clone())
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
                        &project_id,
                        &bb_job_id,
                        &job.bugbounty_finding_ids,
                        event_tx,
                    );
                    bugbounty_job_id = Some(bb_job_id);
                }
                }
            }
        }
    } else {
        match BugBountyManager::new() {
            Ok(bb) => {
                let mut resolved_project_id: Option<String> = None;
                let mut resolved_project_root_abs: Option<std::path::PathBuf> = None;

                // 1) Explicit project id (if provided)
                if let Some(explicit_project_id) = bugbounty_project_id.clone() {
                    match bb.get_project(&explicit_project_id) {
                        Ok(Some(project)) => {
                            let root_raw = std::path::PathBuf::from(&project.root_path);
                            let root_abs = if root_raw.is_absolute() {
                                root_raw
                            } else {
                                bb_work_dir.join(root_raw)
                            };
                            resolved_project_id = Some(explicit_project_id);
                            resolved_project_root_abs = Some(root_abs.canonicalize().unwrap_or(root_abs));
                        }
                        Ok(None) => {
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty project not found: {}",
                                explicit_project_id
                            ))));
                            bugbounty_project_id = None;
                        }
                        Err(err) => {
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty project lookup skipped: {}",
                                err
                            ))));
                            bugbounty_project_id = None;
                        }
                    }
                }

                // 2) Inference fallback (only when no explicit project resolved)
                if resolved_project_id.is_none() {
                    match bb.infer_project_for_path(bb_work_dir, &job.source_file) {
                        Ok(Some((project, root_abs))) => {
                            let project_id = project.id.clone();
                            bugbounty_project_id = Some(project_id.clone());
                            job.bugbounty_project_id = Some(project_id.clone());
                            if let Ok(mut manager) = job_manager.lock() {
                                if let Some(j) = manager.get_mut(job_id) {
                                    j.bugbounty_project_id = Some(project_id.clone());
                                }
                                manager.touch();
                            }
                            resolved_project_id = Some(project_id);
                            resolved_project_root_abs = Some(root_abs);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty project inference skipped: {}",
                                err
                            ))));
                        }
                    }
                }

                if let (Some(project_id), Some(project_root_abs)) =
                    (resolved_project_id, resolved_project_root_abs)
                {
                    let file_rel = job
                        .source_file
                        .strip_prefix(&project_root_abs)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string());

                    let injected_result = if let Some(ref file_rel) = file_rel {
                        ContextInjector::new(bb.clone()).for_file(&project_id, file_rel)
                    } else {
                        ContextInjector::new(bb.clone()).for_project(&project_id)
                    };

                    match injected_result {
                        Ok(mut injected) => {
                            if let Some(focus) = format_focus_findings(
                                &bb,
                                &project_id,
                                &job.bugbounty_finding_ids,
                            ) {
                                injected.focus_findings = Some(focus);
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

                    // Persist a BugBounty job record (used for linking findings/artifacts).
                    let bb_job_id = uuid::Uuid::new_v4().to_string();
                    let mut bb_job = BugBountyJob::new(&bb_job_id)
                        .with_project_id(project_id.clone())
                        .with_kyco_job_id(job_id)
                        .with_mode(job.skill.clone())
                        .mark_started();
                    if let Some(ref file_rel) = file_rel {
                        bb_job = bb_job.with_target_file(file_rel.clone());
                    }
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
                            &project_id,
                            &bb_job_id,
                            &job.bugbounty_finding_ids,
                            event_tx,
                        );
                        bugbounty_job_id = Some(bb_job_id);
                    }
                }
            }
            Err(_) => {
                // No bugbounty DB configured/available - ignore silently.
            }
        }
    }

    {
        let Ok(mut manager) = job_manager.lock() else {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Job #{} failed: lock poisoned",
                job_id
            ))));
            return;
        };
        manager.set_status(job_id, JobStatus::Running);
    }

    let _ = event_tx.send(ExecutorEvent::JobStarted(job_id));
    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
        "Starting job #{}",
        job_id
    ))));

    // Multi-agent jobs always require worktrees for isolation
    let is_multi_agent_job = job.group_id.is_some();

    // Check if the mode/chain has a use_worktree override
    let mode_use_worktree = config
        .mode
        .get(&job.skill)
        .and_then(|m| m.use_worktree)
        .or_else(|| config.chain.get(&job.skill).and_then(|c| c.use_worktree));

    let should_use_worktree = match mode_use_worktree {
        Some(true) => true,   // Mode/chain explicitly enables worktree
        Some(false) => false, // Mode/chain explicitly disables worktree
        None => config.settings.use_worktree || is_multi_agent_job || job.force_worktree,
    };

    // Check if we have a custom workspace different from work_dir (before taking ownership)
    let has_custom_workspace = job.workspace_path.as_ref().is_some_and(|p| p != work_dir);
    // Take ownership of workspace_path or clone work_dir
    let job_work_dir = job
        .workspace_path
        .take()
        .unwrap_or_else(|| work_dir.clone());
    let workspace_root = job_work_dir.clone();

    let job_git_manager = if has_custom_workspace {
        GitManager::new(&job_work_dir).ok()
    } else {
        None
    };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    // Take existing worktree path if it exists and is valid (avoid clone by taking ownership)
    let (worktree_path, is_in_worktree) =
        if let Some(existing_worktree) = job.git_worktree_path.take().filter(|p| p.exists()) {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                "Reusing worktree: {}",
                existing_worktree.display()
            ))));
            (existing_worktree, true)
        } else if should_use_worktree {
            match setup_worktree(
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
                None => return, // Early return on required worktree failure
            }
        } else {
            // No worktree needed - move job_work_dir instead of cloning
            (job_work_dir, false)
        };

    if is_in_worktree {
        let remap = remap_job_paths_to_worktree(&mut job, &workspace_root, &worktree_path);
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

    let mut agent_config = config
        .get_agent_for_job(&job.agent_id, &job.skill)
        .unwrap_or_default();

    // When using a worktree, automatically allow git commands for committing
    if is_in_worktree {
        let git_tools = [
            "git",
            "Bash(git:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git status:*)",
            "Bash(git diff:*)",
            "Bash(git log:*)",
        ];
        for tool in git_tools {
            let tool_str = tool.to_string();
            if !agent_config.allowed_tools.contains(&tool_str) {
                agent_config.allowed_tools.push(tool_str);
            }
        }
    }

    // BugBounty hardening (best-effort): convert ToolPolicy into tool-level blocks.
    if let Some(project_id) = bugbounty_project_id.as_deref() {
        if let Ok(bb) = BugBountyManager::new() {
            if let Ok(Some(project)) = bb.get_project(project_id) {
                // Provide scope/policy to the SDK Bridge via env vars so the tool-layer
                // can enforce allow/deny + protected paths (not just prompt guidance).
                agent_config
                    .env
                    .insert("KYCO_BUGBOUNTY_ENFORCE".to_string(), "1".to_string());
                agent_config.env.insert(
                    "KYCO_BUGBOUNTY_PROJECT_ID".to_string(),
                    project.id.clone(),
                );

                let root_raw = std::path::PathBuf::from(&project.root_path);
                let root_abs = if root_raw.is_absolute() {
                    if is_in_worktree {
                        root_raw
                            .strip_prefix(work_dir)
                            .map(|rel| worktree_path.join(rel))
                            .unwrap_or(root_raw)
                    } else {
                        root_raw
                    }
                } else if is_in_worktree {
                    worktree_path.join(&root_raw)
                } else {
                    work_dir.join(&root_raw)
                };
                let root_abs = root_abs.canonicalize().unwrap_or(root_abs);
                agent_config.env.insert(
                    "KYCO_BUGBOUNTY_PROJECT_ROOT".to_string(),
                    root_abs.to_string_lossy().to_string(),
                );

                if let Some(ref scope) = project.scope {
                    if let Ok(json) = serde_json::to_string(scope) {
                        agent_config
                            .env
                            .insert("KYCO_BUGBOUNTY_SCOPE_JSON".to_string(), json);
                    }
                }
                if let Some(ref policy) = project.tool_policy {
                    if let Ok(json) = serde_json::to_string(policy) {
                        agent_config.env.insert(
                            "KYCO_BUGBOUNTY_TOOL_POLICY_JSON".to_string(),
                            json,
                        );
                    }
                }

                if let Some(policy) = project.tool_policy {
                    let mut blocked = policy.blocked_commands.clone();
                    if policy.network_wrapper.is_some() {
                        for cmd in ["curl", "wget", "nc", "nmap"] {
                            if !blocked.iter().any(|c| c.eq_ignore_ascii_case(cmd)) {
                                blocked.push(cmd.to_string());
                            }
                        }
                    }

                    for cmd in &blocked {
                        let cmd = cmd.trim();
                        if cmd.is_empty() {
                            continue;
                        }
                        let pattern = format!("Bash({}:*)", cmd);
                        if !agent_config.disallowed_tools.contains(&pattern) {
                            agent_config.disallowed_tools.push(pattern);
                        }
                    }
                }
            }
        }
    }

    // All agents now use persistent sessions (SessionMode removed)
    let is_repl = true;
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            j.is_repl = is_repl;
        }
    }

    let adapter = match agent_registry.get_for_config(&agent_config) {
        Some(a) => a,
        None => {
            let error = format!("No adapter found for agent '{}'", job.agent_id);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    let (log_tx, log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);
    let log_forwarder =
        spawn_log_forwarder(log_rx, event_tx.clone(), Arc::clone(job_manager), job_id);

    // Track git stats info for async calculation after lock release
    let mut git_stats_info: Option<(usize, Option<String>)> = None;

    match adapter
        .run(&job, &worktree_path, &agent_config, log_tx)
        .await
    {
        Ok(mut result) => {
            let mut bugbounty_ctx: Option<crate::bugbounty::NextContext> = None;
            let mut bugbounty_next_context_value: Option<serde_json::Value> = None;
            let mut bugbounty_result_state: Option<String> = None;

            // Extract structured next_context for BugBounty ingestion without cloning the full output.
            let mut output_text = result.output_text.take();
            if bugbounty_project_id.is_some() {
                if let Some(ref output) = output_text {
                    // Preferred: use the parsed job result's `next_context` (supports nested YAML under ---).
                    if let Some(job_result) = crate::JobResult::parse(output) {
                        bugbounty_result_state = job_result.state.clone();
                        if let Some(ref value) = job_result.next_context {
                            bugbounty_next_context_value = Some(value.clone());
                        }
                        if let Some(value) = job_result.next_context {
                            if let Ok(ctx) = crate::bugbounty::NextContext::from_value(value) {
                                if !ctx.is_empty() {
                                    bugbounty_ctx = Some(ctx);
                                }
                            }
                        }
                    }

                    // Fallback: accept standalone next_context blocks (json/yaml) in the raw output.
                    if bugbounty_ctx.is_none() {
                        if let Some(ctx) = crate::bugbounty::NextContext::extract_from_text(output) {
                            if !ctx.is_empty() {
                                bugbounty_ctx = Some(ctx);
                            }
                        }
                    }
                }
            }

            // Strict output contract enforcement (security-audit profile).
            // Only enabled for explicit security skills (or when the user linked a finding).
            let enforce_bugbounty_contract = bugbounty_project_id.is_some()
                && (is_bugbounty_security_skill(&job.skill) || !job.bugbounty_finding_ids.is_empty());
            if enforce_bugbounty_contract && result.success {
                match bugbounty_ctx.as_ref() {
                    Some(ctx) => {
                        if let Err(err) = ctx.validate_security_audit() {
                            let msg = format!("BugBounty output contract violated: {}", err);
                            let _ = event_tx
                                .send(ExecutorEvent::Log(LogEvent::error(msg.clone())));
                            result.success = false;
                            result.error = Some(msg);
                            bugbounty_ctx = None;
                        }
                    }
                    None => {
                        let msg =
                            "BugBounty output contract violated: missing next_context block".to_string();
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(msg.clone())));
                        result.success = false;
                        result.error = Some(msg);
                    }
                }
            }

            {
                let Ok(mut manager) = job_manager.lock() else {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Job #{} completed but lock poisoned",
                        job_id
                    ))));
                    return;
                };
                if let Some(j) = manager.get_mut(job_id) {
                    let was_cancel_requested = j.cancel_requested;
                    // Use take() to move values instead of cloning
                    j.sent_prompt = result.sent_prompt.take();

                    // Copy token usage from agent result (primitives, no allocation)
                    j.input_tokens = result.input_tokens;
                    j.output_tokens = result.output_tokens;
                    j.cache_read_tokens = result.cache_read_tokens;
                    j.cache_write_tokens = result.cache_write_tokens;
                    j.cost_usd = result.cost_usd;

                    // Take output_text to avoid clone; parse_result only needs a reference
                    if let Some(output) = output_text.take() {
                        j.parse_result(&output);
                        j.full_response = Some(output);
                    }

                    // Move session_id instead of cloning
                    j.bridge_session_id = result.session_id.take();

                    // Restore worktree path so continuation jobs can reuse it
                    // (it was taken earlier with .take() to avoid cloning)
                    if is_in_worktree {
                        j.git_worktree_path = Some(worktree_path.clone());
                    }

                    let files_changed = result.changed_files.len();
                    // Store info for async git stats calculation after lock release
                    if files_changed > 0 && is_in_worktree {
                        git_stats_info = Some((files_changed, j.base_branch.clone()));
                    }

                    if result.success {
                        j.set_status(JobStatus::Done);
                        j.changed_files = result.changed_files;
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Job #{} completed",
                            job_id
                        ))));
                        let _ = event_tx.send(ExecutorEvent::JobCompleted(job_id));
                    } else {
                        let error = if was_cancel_requested {
                            "Job aborted by user".to_string()
                        } else {
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        };
                        j.fail(error.clone());
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Job #{} failed: {}",
                            job_id, error
                        ))));
                        let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
                    }
                }
                manager.touch();
            }

            // BugBounty ingestion happens after the JobManager lock is released.
            if let Some(project_id) = bugbounty_project_id.as_deref() {
                let fallback_job_id = job_id.to_string();
                let bb_job_id = bugbounty_job_id.as_deref().unwrap_or(&fallback_job_id);

                match BugBountyManager::new() {
                    Ok(bb) => {
                        // Ensure the job row exists so FK constraints are satisfied (artifacts/job_findings).
                        let _ = bb.jobs().ensure_exists(bb_job_id, Some(project_id));

                        if let Some(ctx) = bugbounty_ctx {
                            match bb.process_next_context(project_id, &ctx, Some(bb_job_id)) {
                                Ok(finding_ids) => {
                                    if !finding_ids.is_empty() {
                                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(
                                            format!(
                                                "BugBounty: stored {} finding(s) for project {}",
                                                finding_ids.len(),
                                                project_id
                                            ),
                                        )));
                                    }
                                }
                                Err(err) => {
                                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(
                                        format!("BugBounty ingestion failed: {}", err),
                                    )));
                                }
                            }
                        }

                        // Persist completion metadata (even if no next_context was emitted).
                        let completed_at = chrono::Utc::now().timestamp_millis();
                        let status = if result.success { "done" } else { "failed" };
                        if let Err(err) = bb.jobs().mark_completed(
                            bb_job_id,
                            status,
                            completed_at,
                            bugbounty_result_state.as_deref(),
                            bugbounty_next_context_value.as_ref(),
                        ) {
                            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                                "BugBounty job update failed: {}",
                                err
                            ))));
                        }
                    }
                    Err(err) => {
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "BugBounty ingestion skipped: {}",
                            err
                        ))));
                    }
                }
            }
        }
        Err(e) => {
            let mut error = e.to_string();
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    if j.cancel_requested {
                        error = "Job aborted by user".to_string();
                    }
                    j.fail(error.clone());
                    // Restore worktree path for potential retry/continuation
                    if is_in_worktree {
                        j.git_worktree_path = Some(worktree_path.clone());
                    }
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Job #{} error: {}",
                job_id, error
            ))));
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
        }
    }

    // Calculate git stats asynchronously after releasing the lock
    // This avoids blocking the async runtime with synchronous git operations
    if let Some((files_changed, base_branch)) = git_stats_info {
        let (lines_added, lines_removed) =
            calculate_git_numstat_async(&worktree_path, base_branch.as_deref()).await;

        // Re-acquire lock to update file stats
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.set_file_stats(files_changed, lines_added, lines_removed);
            }
            manager.touch();
        }
    }

    let _ = log_forwarder.await;
}
