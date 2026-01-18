//! Codex CLI agent adapter

mod parser;

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::runner::{AgentResult, AgentRunner};
use crate::agent::process_registry;
use crate::{AgentConfig, Job, LogEvent};
use parser::{CodexEventResult, parse_codex_event};

/// Codex CLI agent adapter
///
/// Codex CLI uses `codex exec --json "prompt"` for non-interactive mode.
/// Output format differs from Claude Code.
pub struct CodexAdapter {
    id: String,
}

impl CodexAdapter {
    /// Create a new Codex adapter
    pub fn new() -> Self {
        Self {
            id: "codex".to_string(),
        }
    }

    /// Build the prompt for a job using the mode template from config
    fn build_prompt(&self, job: &Job, worktree: &Path, config: &AgentConfig) -> String {
        let template = config.get_skill_template(&job.skill);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");

        let ide_context = job.ide_context.as_deref().unwrap_or("");
        let base_prompt = template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.skill)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
            .replace("{ide_context}", ide_context);

        let mut prompt = String::new();

        let mut system_prompt = template.system_prompt.clone().unwrap_or_default();
        if job.git_worktree_path.is_some() {
            system_prompt.push_str("\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.");
        }
        if !system_prompt.trim().is_empty() {
            prompt.push_str("## System Instructions\n\n");
            prompt.push_str(system_prompt.trim());
            prompt.push_str("\n\n");
        }

        // If a SKILL.md exists for this mode, prefer it (Codex doesn't auto-load skills reliably).
        if let Some(skill_md) = find_skill_md_path(job, worktree) {
            prompt.push_str("## Skill\n\n");
            prompt.push_str(&format!("Source: `{}`\n\n", skill_md.display()));
            match std::fs::read_to_string(&skill_md) {
                Ok(content) if !content.trim().is_empty() => {
                    prompt.push_str(content.trim_end());
                    prompt.push_str("\n\n");
                }
                _ => {
                    prompt.push_str("Unable to read skill content. You may open the file path above if needed.\n\n");
                }
            }
        }

        prompt.push_str("## Task\n\n");
        prompt.push_str(&base_prompt);
        prompt
    }

    fn build_args(&self, job: &Job, worktree: &Path, config: &AgentConfig) -> Vec<String> {
        // Note: `--ask-for-approval` is a *global* Codex flag (must come before `exec`).
        let mut args: Vec<String> = Vec::new();

        // Keep Codex non-interactive by default. Users can opt into the global Codex config.
        if config.allow_dangerous_bypass {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        } else {
            args.push("--ask-for-approval".to_string());
            args.push(
                config
                    .ask_for_approval
                    .clone()
                    .unwrap_or_else(|| "never".to_string()),
            );

            args.push("--sandbox".to_string());
            args.push(
                config
                    .sandbox
                    .clone()
                    .unwrap_or_else(|| "workspace-write".to_string()),
            );
        }

        args.push("exec".to_string());
        args.push("--json".to_string());

        if let Some(model) = config.model.as_deref() {
            args.push("--model".to_string());
            args.push(model.to_string());
        }

        // Always allow running outside a git repo (e.g., temp dirs in tests).
        args.push("--skip-git-repo-check".to_string());

        if let Some(add_dir) = find_skill_add_dir(job, worktree) {
            args.push("--add-dir".to_string());
            args.push(add_dir.display().to_string());
        }

        let is_resume = job.bridge_session_id.is_some();
        if is_resume {
            args.push("resume".to_string());
            args.push(job.bridge_session_id.clone().unwrap_or_default());
        } else {
            args.push("--cd".to_string());
            args.push(worktree.display().to_string());
        }

        // Provide the prompt via stdin to avoid command line length limits.
        args.push("-".to_string());
        args
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for CodexAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        struct ProcessGuard {
            job_id: u64,
            registered: bool,
        }
        impl ProcessGuard {
            fn register(job_id: u64, pid: Option<u32>, agent_id: &str) -> Self {
                if let Some(pid) = pid {
                    process_registry::register(job_id, pid, agent_id);
                    return Self {
                        job_id,
                        registered: true,
                    };
                }
                Self {
                    job_id,
                    registered: false,
                }
            }
        }
        impl Drop for ProcessGuard {
            fn drop(&mut self) {
                if self.registered {
                    process_registry::unregister(self.job_id);
                }
            }
        }

        let prompt = self.build_prompt(job, worktree, config);
        let args = self.build_args(job, worktree, config);

        let job_id = job.id;
        let _ = event_tx
            .send(
                LogEvent::system(format!("Starting job #{} with prompt:", job_id)).for_job(job_id),
            )
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        let binary = config.get_binary();
        let mut child = Command::new(&binary)
            .args(&args)
            .current_dir(worktree)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env)
            .spawn()
            .with_context(|| format!("Failed to spawn {}", binary))?;

        let _process_guard = ProcessGuard::register(job_id, child.id(), self.id());

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await.ok();
        }

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture stdout pipe")?;
        let stderr = child
            .stderr
            .take()
            .context("Failed to capture stderr pipe")?;
        let mut reader = BufReader::new(stdout).lines();

        let event_tx_clone = event_tx.clone();
        let stderr_task = tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                let _ = event_tx_clone
                    .send(LogEvent::error(format!("stderr: {}", line)).for_job(job_id))
                    .await;
            }
        });

        let mut result = AgentResult {
            success: false,
            error: None,
            changed_files: Vec::new(),
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            cache_read_tokens: None,
            cache_write_tokens: None,
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
            output_text: None,
            structured_output: None,
            session_id: job.bridge_session_id.clone(),
        };

        // Track if we received turn.completed (means success regardless of exit code)
        let mut turn_completed = false;
        let mut output_text = String::new();

        while let Ok(Some(line)) = reader.next_line().await {
            match parse_codex_event(&line) {
                CodexEventResult::ThreadStarted { thread_id } => {
                    result.session_id = Some(thread_id.clone());
                    let _ = event_tx
                        .send(
                            LogEvent::system("Codex thread started")
                                .with_tool_args(serde_json::json!({ "session_id": thread_id }))
                                .for_job(job_id),
                        )
                        .await;
                }
                CodexEventResult::TurnCompleted {
                    input_tokens,
                    cached_input_tokens,
                    output_tokens,
                } => {
                    turn_completed = true;
                    result.success = true;
                    let fresh_input_tokens = input_tokens.saturating_sub(cached_input_tokens);
                    result.input_tokens = Some(fresh_input_tokens);
                    result.output_tokens = Some(output_tokens);
                    result.cache_read_tokens = Some(cached_input_tokens);
                    let _ = event_tx
                        .send(
                            LogEvent::system(format!(
                                "Completed (tokens: {} in, {} cached, {} fresh, {} out)",
                                input_tokens, cached_input_tokens, fresh_input_tokens, output_tokens
                            ))
                            .for_job(job_id),
                        )
                        .await;
                }
                CodexEventResult::AssistantMessage { text } => {
                    output_text.push_str(&text);
                    output_text.push('\n');
                    let first_line = text.lines().next().unwrap_or("");
                    if !first_line.is_empty() {
                        let _ = event_tx
                            .send(LogEvent::text(first_line).for_job(job_id))
                            .await;
                    }
                }
                CodexEventResult::FilesChanged { paths } => {
                    for p in &paths {
                        result.changed_files.push(p.clone());
                    }
                    let _ = event_tx
                        .send(
                            LogEvent::tool_output(
                                "files",
                                format!("Changed {} file(s)", paths.len()),
                            )
                            .for_job(job_id),
                        )
                        .await;
                }
                CodexEventResult::Log(event) => {
                    let _ = event_tx.send(event.for_job(job_id)).await;
                }
                CodexEventResult::None => {}
            }
        }

        let status = child.wait().await?;

        // Wait for stderr task to complete to ensure all logs are captured
        // before sending completion message (prevents race condition in log ordering)
        let _ = stderr_task.await;

        // Success is based on turn.completed, not exit code
        // Codex may exit with code 1 even on success (e.g., if tests fail)
        if turn_completed {
            result.success = true;
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
                .await;
        } else if status.success() {
            result.success = true;
            let _ = event_tx
                .send(LogEvent::system(format!("Job #{} completed", job_id)).for_job(job_id))
                .await;
        } else {
            result.error = Some(format!("Process exited with status: {}", status));
            let _ = event_tx
                .send(
                    LogEvent::error(format!("Job #{} failed: {}", job_id, status)).for_job(job_id),
                )
                .await;
        }

        if !output_text.is_empty() {
            result.output_text = Some(output_text);
        }

        Ok(result)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg("codex")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

fn find_skill_md_path(job: &Job, worktree: &Path) -> Option<std::path::PathBuf> {
    let skill = job.skill.as_str();
    let mut candidates = Vec::new();

    candidates.push(worktree.join(".codex/skills").join(skill).join("SKILL.md"));
    candidates.push(worktree.join(".codex/skills").join(format!("{}.md", skill)));

    if let Some(workspace) = job.workspace_path.as_ref() {
        candidates.push(workspace.join(".codex/skills").join(skill).join("SKILL.md"));
        candidates.push(
            workspace
                .join(".codex/skills")
                .join(format!("{}.md", skill)),
        );
    }

    if let Some(worktree) = job.git_worktree_path.as_ref() {
        candidates.push(worktree.join(".codex/skills").join(skill).join("SKILL.md"));
        candidates.push(worktree.join(".codex/skills").join(format!("{}.md", skill)));
    }

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".codex/skills").join(skill).join("SKILL.md"));
        candidates.push(home.join(".codex/skills").join(format!("{}.md", skill)));
        candidates.push(home.join(".kyco/skills").join(skill).join("SKILL.md"));
        candidates.push(home.join(".kyco/skills").join(format!("{}.md", skill)));
    }

    for path in candidates {
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn find_skill_add_dir(job: &Job, worktree: &Path) -> Option<std::path::PathBuf> {
    let skill = job.skill.as_str();
    let mut candidates = Vec::new();

    if let Some(workspace) = job.workspace_path.as_ref() {
        candidates.push(workspace.join(".codex/skills").join(skill));
        candidates.push(workspace.join(".codex/skills"));
    }

    candidates.push(worktree.join(".codex/skills").join(skill));
    candidates.push(worktree.join(".codex/skills"));

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".codex/skills").join(skill));
        candidates.push(home.join(".codex/skills"));
        candidates.push(home.join(".kyco/skills").join(skill));
        candidates.push(home.join(".kyco/skills"));
    }

    for path in candidates {
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}
