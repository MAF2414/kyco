//! Claude Code agent adapter

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::output::{ContentBlock, StreamEvent};
use crate::agent::runner::{AgentResult, AgentRunner};
use crate::{AgentConfig, Job, LogEvent};

/// Claude Code agent adapter
pub struct ClaudeAdapter {
    id: String,
}

impl ClaudeAdapter {
    /// Create a new Claude adapter
    pub fn new() -> Self {
        Self {
            id: "claude".to_string(),
        }
    }

    /// Build the prompt for a job using the mode template from config
    fn build_prompt(&self, job: &Job, config: &AgentConfig) -> String {
        let template = config.get_mode_template(&job.mode);
        let file_path = job.source_file.display().to_string();
        let line = job.source_line;
        let description = job.description.as_deref().unwrap_or("");

        // Replace template placeholders
        let ide_context = job.ide_context.as_deref().unwrap_or("");
        template
            .prompt_template
            .replace("{file}", &file_path)
            .replace("{line}", &line.to_string())
            .replace("{target}", &job.target)
            .replace("{mode}", &job.mode)
            .replace("{description}", description)
            .replace("{scope_type}", "file")
            .replace("{ide_context}", ide_context)
    }

    /// Build the system prompt addition for a job
    fn build_system_prompt(&self, job: &Job, config: &AgentConfig) -> Option<String> {
        let template = config.get_mode_template(&job.mode);
        let mut system_prompt = template.system_prompt.unwrap_or_default();

        // If running in a worktree, add commit instruction
        if job.git_worktree_path.is_some() {
            let commit_instruction = "\n\nIMPORTANT: You are working in an isolated Git worktree. When you have completed the task, commit all your changes with a descriptive commit message. Do NOT push.";
            system_prompt.push_str(commit_instruction);
        }

        // Append output schema if configured
        if let Some(schema) = &config.output_schema {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(schema);
        }

        if system_prompt.is_empty() {
            None
        } else {
            Some(system_prompt)
        }
    }

    /// Build command arguments
    fn build_args(&self, job: &Job, config: &AgentConfig, prompt: &str) -> Vec<String> {
        let mut args = config.get_run_args();

        // Add system prompt if configured
        if let Some(system_prompt) = self.build_system_prompt(job, config) {
            match config.system_prompt_mode {
                crate::SystemPromptMode::Append => {
                    args.push("--append-system-prompt".to_string());
                    args.push(system_prompt);
                }
                crate::SystemPromptMode::Replace => {
                    args.push("--system-prompt".to_string());
                    args.push(system_prompt);
                }
                crate::SystemPromptMode::ConfigOverride => {
                    // ConfigOverride not applicable for Claude - treat as append
                    args.push("--append-system-prompt".to_string());
                    args.push(system_prompt);
                }
            }
        }

        // Add disallowed tools
        if !config.disallowed_tools.is_empty() {
            args.push("--disallowedTools".to_string());
            args.push(config.disallowed_tools.join(","));
        }

        // Add allowed tools
        if !config.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            args.push(config.allowed_tools.join(","));
        }

        // Add -- separator to indicate end of flags
        args.push("--".to_string());

        // Add the prompt at the end
        args.push(prompt.to_string());

        args
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for ClaudeAdapter {
    async fn run(
        &self,
        job: &Job,
        worktree: &Path,
        config: &AgentConfig,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> Result<AgentResult> {
        let prompt = self.build_prompt(job, config);
        let args = self.build_args(job, config, &prompt);

        // Send start event with full prompt
        let job_id = job.id;
        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting job #{} with prompt:",
                job_id
            )).for_job(job_id))
            .await;
        let _ = event_tx
            .send(LogEvent::system(format!(">>> {}", prompt)).for_job(job_id))
            .await;

        // Spawn the process
        let mut child = Command::new(&config.binary)
            .args(&args)
            .current_dir(worktree)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.env)
            .spawn()
            .with_context(|| format!("Failed to spawn {}", config.binary))?;

        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");
        let mut reader = BufReader::new(stdout).lines();

        // Spawn a task to read stderr
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
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
            duration_ms: None,
            sent_prompt: Some(prompt.clone()),
            output_text: None,
        };

        // Collect text output for parsing
        let mut output_text = String::new();

        // Process output stream
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(event) = StreamEvent::parse(&line) {
                let log_event = match &event {
                    StreamEvent::System { subtype, message } => {
                        LogEvent::system(format!("{}: {}", subtype, message.as_deref().unwrap_or("")))
                    }
                    StreamEvent::Assistant { message } => {
                        let mut events = Vec::new();
                        for block in &message.content {
                            match block {
                                ContentBlock::Text { text } => {
                                    // Collect text output for parsing ---kyco blocks
                                    output_text.push_str(text);
                                    output_text.push('\n');
                                    events.push(LogEvent::text(text.clone()));
                                }
                                ContentBlock::ToolUse { name, input, .. } => {
                                    let summary = format_tool_call(name, input);
                                    events.push(LogEvent::tool_call(name.clone(), summary));
                                }
                                _ => {}
                            }
                        }
                        // Send all but the last, return the last
                        for evt in events.drain(..events.len().saturating_sub(1)) {
                            let _ = event_tx.send(evt.for_job(job_id)).await;
                        }
                        events.pop().unwrap_or_else(|| LogEvent::system("assistant message"))
                    }
                    StreamEvent::User { message } => {
                        let mut summary = String::new();
                        for block in &message.content {
                            if let ContentBlock::ToolResult { content, is_error, .. } = block {
                                summary = if *is_error {
                                    format!("Error: {}", content)
                                } else {
                                    content.clone()
                                };
                            }
                        }
                        LogEvent::tool_output("tool", summary)
                    }
                    StreamEvent::Result {
                        subtype,
                        cost_usd,
                        duration_ms,
                        ..
                    } => {
                        result.cost_usd = *cost_usd;
                        result.duration_ms = *duration_ms;

                        if subtype == "success" {
                            result.success = true;
                        }

                        LogEvent::system(format!(
                            "Completed: {} (cost: ${:.4}, duration: {}ms)",
                            subtype,
                            cost_usd.unwrap_or(0.0),
                            duration_ms.unwrap_or(0)
                        ))
                    }
                };

                let _ = event_tx.send(log_event.for_job(job_id)).await;
            }
        }

        // Wait for the process to finish
        let status = child.wait().await?;

        if !status.success() && !result.success {
            result.error = Some(format!("Process exited with status: {}", status));
        }

        // Set collected output text for parsing
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
            .arg("claude")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Format a tool call for display
fn format_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Read {}", path)
            } else {
                "Read file".to_string()
            }
        }
        "Write" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Write {}", path)
            } else {
                "Write file".to_string()
            }
        }
        "Edit" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Edit {}", path)
            } else {
                "Edit file".to_string()
            }
        }
        "Bash" => {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                format!("Bash: {}", cmd)
            } else {
                "Bash command".to_string()
            }
        }
        "Glob" => {
            if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                format!("Glob: {}", pattern)
            } else {
                "Glob search".to_string()
            }
        }
        "Grep" => {
            if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                format!("Grep: {}", pattern)
            } else {
                "Grep search".to_string()
            }
        }
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Job, ScopeDefinition};
    use std::path::PathBuf;

    fn create_test_job(
        mode: &str,
        description: Option<&str>,
        source_file: &str,
        source_line: usize,
    ) -> Job {
        // Target format matches what JobManager creates: file:line
        let target = format!("{}:{}", source_file, source_line);
        Job::new(
            1,
            mode.to_string(),
            ScopeDefinition::file(PathBuf::from(source_file)),
            target,
            description.map(|s| s.to_string()),
            "claude".to_string(),
            PathBuf::from(source_file),
            source_line,
            None, // raw_tag_line not needed for these tests
        )
    }

    #[test]
    fn test_prompt_includes_file_and_line() {
        let adapter = ClaudeAdapter::new();
        let config = AgentConfig::default();
        let job = create_test_job("refactor", Some("fix the bug"), "src/main.rs", 42);

        let prompt = adapter.build_prompt(&job, &config);

        // Must contain file:line reference
        assert!(prompt.contains("src/main.rs:42"), "Prompt should contain file:line reference");
        // Must contain description
        assert!(prompt.contains("fix the bug"), "Prompt should contain description");
    }

    #[test]
    fn test_prompt_without_description() {
        let adapter = ClaudeAdapter::new();
        let config = AgentConfig::default();
        let job = create_test_job("refactor", None, "lib/utils.py", 10);

        let prompt = adapter.build_prompt(&job, &config);

        // Must contain file:line reference
        assert!(prompt.contains("lib/utils.py:10"), "Prompt should contain file:line reference");
        // Must mention the mode (case-insensitive, template uses "Refactor")
        assert!(prompt.to_lowercase().contains("refactor"), "Prompt should mention the mode");
    }

    #[test]
    fn test_prompt_different_files() {
        let adapter = ClaudeAdapter::new();
        let config = AgentConfig::default();

        // Test with different file paths
        let test_cases = vec![
            ("src/app.tsx", 1, "src/app.tsx:1"),
            ("./relative/path.rs", 100, "./relative/path.rs:100"),
            ("deep/nested/file.go", 50, "deep/nested/file.go:50"),
        ];

        for (file, line, expected) in test_cases {
            let job = create_test_job("implement", Some("do something"), file, line);
            let prompt = adapter.build_prompt(&job, &config);
            assert!(
                prompt.contains(expected),
                "Prompt should contain '{}', got: {}",
                expected,
                prompt
            );
        }
    }

    #[test]
    fn test_prompt_format_with_description() {
        let adapter = ClaudeAdapter::new();
        let config = AgentConfig::default();
        let job = create_test_job("fix", Some("handle edge cases"), "test.rs", 5);

        let prompt = adapter.build_prompt(&job, &config);

        // Prompt should contain target (file:line) and description
        assert!(prompt.contains("test.rs:5"), "Prompt should contain file:line reference");
        assert!(prompt.contains("handle edge cases"), "Prompt should contain description");
    }

    #[test]
    fn test_prompt_format_without_description() {
        let adapter = ClaudeAdapter::new();
        let config = AgentConfig::default();
        let job = create_test_job("tests", None, "code.py", 20);

        let prompt = adapter.build_prompt(&job, &config);

        // Format should mention file:line and mode
        assert!(prompt.contains("code.py:20"), "Prompt should contain file:line");
        assert!(prompt.contains("code.py"), "Prompt should mention file");
        // "tests" mode has a template with "Write unit tests"
        assert!(prompt.contains("test"), "Prompt should mention tests");
    }
}
