//! Codex bridge adapter implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::sync::mpsc;

use super::super::client::BridgeClient;
use super::super::types::*;
use super::util::{bridge_cwd, extract_output_from_result, format_tool_call, parse_json_schema, resolve_prompt_paths};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

/// Codex adapter using the SDK Bridge
pub struct CodexBridgeAdapter {
    client: BridgeClient,
}

impl CodexBridgeAdapter {
    pub fn new() -> Self {
        Self { client: BridgeClient::new() }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self { client: BridgeClient::with_url(url) }
    }

    pub(super) fn build_prompt(&self, job: &Job, config: &AgentConfig, _worktree: &Path) -> String {
        let template = config.get_mode_template(&job.mode);
        let paths = resolve_prompt_paths(job);

        let mut prompt = template.prompt_template
            .replace("{file}", &paths.file_path)
            .replace("{line}", &job.source_line.to_string())
            .replace("{target}", &paths.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", job.description.as_deref().unwrap_or(""))
            .replace("{scope_type}", "file")
            .replace("{ide_context}", &paths.ide_context);

        if let Some(system_prompt) = template.system_prompt.as_deref() {
            let system_prompt = system_prompt.trim();
            if !system_prompt.is_empty() {
                prompt = format!("{}\n\n{}", system_prompt, prompt);
            }
        }

        if let Some(wt_path) = &job.git_worktree_path {
            prompt.push_str(&format!(
                "\n\n---\n**IMPORTANT: Working Directory**\n\
                You are working in an isolated Git worktree at: `{}`\n\
                All file paths are relative to this worktree. \
                Do NOT edit files outside this directory. \
                When done, commit your changes with a descriptive message. Do NOT push.",
                wt_path.display()
            ));
        }

        if let Some(schema) = &config.output_schema {
            if !schema.trim().is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(schema);
            }
        }

        if let Some(ref state_prompt) = template.state_prompt {
            if !state_prompt.trim().is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(state_prompt);
            }
        } else if !template.output_states.is_empty() {
            prompt.push_str("\n\n## Output State\nWhen you complete this task, indicate the outcome by stating one of the following:\n");
            for state in &template.output_states {
                prompt.push_str(&format!("- state: {}\n", state));
            }
            prompt.push_str(&format!("\nExample: \"state: {}\" (at the end of your response)", &template.output_states[0]));
        }

        prompt
    }
}

impl Default for CodexBridgeAdapter {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl AgentRunner for CodexBridgeAdapter {
    async fn run(&self, job: &Job, worktree: &Path, config: &AgentConfig, event_tx: mpsc::Sender<LogEvent>) -> Result<AgentResult> {
        let job_id = job.id;
        let prompt = self.build_prompt(job, config, worktree);
        let cwd = bridge_cwd(worktree);

        let _ = event_tx.send(LogEvent::system(format!("Starting Codex SDK job #{}", job_id)).for_job(job_id)).await;

        let request = CodexQueryRequest {
            prompt: prompt.clone(),
            images: None,
            cwd,
            thread_id: job.bridge_session_id.clone(),
            sandbox: config.sandbox.clone().or_else(|| Some("workspace-write".to_string())),
            env: if config.env.is_empty() { None } else { Some(config.env.clone()) },
            output_schema: parse_json_schema(config.structured_output_schema.as_deref()),
            model: None, effort: None, approval_policy: None, skip_git_repo_check: None,
        };

        let mut result = AgentResult {
            success: false, error: None, changed_files: Vec::new(), cost_usd: None,
            duration_ms: None, sent_prompt: Some(prompt), output_text: None, session_id: None,
        };

        let mut output_text = String::new();
        let mut captured_session_id: Option<String> = None;
        let mut structured_result: Option<serde_json::Value> = None;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<BridgeEvent, String>>(100);
        let client = self.client.clone();

        tokio::task::spawn_blocking(move || match client.codex_query(&request) {
            Ok(events) => { for ev in events { if tx.blocking_send(ev.map_err(|e| e.to_string())).is_err() { break; } } }
            Err(e) => { let _ = tx.blocking_send(Err(e.to_string())); }
        });

        while let Some(event_result) = rx.recv().await {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => { let _ = event_tx.send(LogEvent::error(format!("Event error: {}", e)).for_job(job_id)).await; continue; }
            };

            match event {
                BridgeEvent::SessionStart { session_id, .. } => {
                    captured_session_id = Some(session_id.clone());
                    let _ = event_tx.send(LogEvent::system("Codex thread started").with_tool_args(serde_json::json!({ "session_id": session_id })).for_job(job_id)).await;
                }
                BridgeEvent::Text { content, partial, .. } => {
                    if !partial { output_text.push_str(&content); output_text.push('\n'); }
                    let _ = event_tx.send(LogEvent::text(content).for_job(job_id)).await;
                }
                BridgeEvent::ToolUse { tool_name, tool_input, .. } => {
                    let _ = event_tx.send(LogEvent::tool_call(tool_name.clone(), format_tool_call(&tool_name, &tool_input)).for_job(job_id)).await;
                }
                BridgeEvent::ToolResult { output, files_changed, .. } => {
                    if let Some(files) = files_changed { for f in files { result.changed_files.push(std::path::PathBuf::from(f)); } }
                    let _ = event_tx.send(LogEvent::tool_output("tool", output).for_job(job_id)).await;
                }
                BridgeEvent::Error { message, .. } => {
                    result.error = Some(message.clone());
                    let _ = event_tx.send(LogEvent::error(message).for_job(job_id)).await;
                }
                BridgeEvent::SessionComplete { success, duration_ms, usage, result: sr, .. } => {
                    result.success = success; result.duration_ms = Some(duration_ms); structured_result = sr;
                    let usage_info = usage.map(|u| format!(", {} tokens", u.input_tokens + u.output_tokens)).unwrap_or_default();
                    let _ = event_tx.send(LogEvent::system(format!("Completed: {} (duration: {}ms{})", if success { "success" } else { "failed" }, duration_ms, usage_info)).for_job(job_id)).await;
                }
                _ => {}
            }
        }

        if !output_text.is_empty() { result.output_text = Some(output_text); }
        extract_output_from_result(&mut result.output_text, structured_result);
        result.session_id = captured_session_id.or_else(|| job.bridge_session_id.clone());

        Ok(result)
    }

    fn id(&self) -> &str { "codex" }
    fn is_available(&self) -> bool { self.client.health_check().is_ok() }
}

#[cfg(test)]
mod tests {
    use super::CodexBridgeAdapter;
    use crate::{AgentConfig, Job, ScopeDefinition};
    use std::path::{Path, PathBuf};

    fn create_test_job(mode: &str, description: Option<&str>, source_file: &str, source_line: usize) -> Job {
        Job::new(1, mode.to_string(), ScopeDefinition::file(PathBuf::from(source_file)), format!("{}:{}", source_file, source_line),
            description.map(|s| s.to_string()), "codex".to_string(), PathBuf::from(source_file), source_line, None)
    }

    #[test]
    fn codex_build_prompt_includes_mode_system_prompt() {
        let adapter = CodexBridgeAdapter::new();
        let config = AgentConfig::codex_default();
        let job = create_test_job("refactor", Some("fix the bug"), "src/main.rs", 42);
        let prompt = adapter.build_prompt(&job, &config, Path::new("."));

        assert!(prompt.starts_with("You are running in KYCo 'refactor' mode."), "Expected mode system prompt prefix, got: {}", prompt);
        assert!(prompt.contains("Refactor the file"), "Expected user prompt to follow system prompt, got: {}", prompt);
    }
}
