//! Orchestrator functionality for KycoApp
//!
//! Contains the orchestrator system prompt and launch logic.

use super::app::KycoApp;
use crate::LogEvent;
use crate::agent::TerminalSession;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default system prompt for the orchestrator
pub(crate) const ORCHESTRATOR_SYSTEM_PROMPT: &str = r#"
You are an interactive KYCo Orchestrator running in the user's workspace.

Your job is to help the user run KYCo jobs (modes/chains) safely and iteratively.

Rules
- Do NOT directly edit repository files yourself. Use KYCo jobs so the user can review diffs in the KYCo GUI.
- Use the `Bash` tool to run `kyco ...` commands.
- Before starting a large batch of jobs, confirm the plan with the user.
- Before changing `.kyco/config.toml` (mode CRUD), ask for explicit confirmation.

Discovery
- List available agents: `kyco agent list`
- List available modes: `kyco mode list`
- List available chains: `kyco chain list`

Job lifecycle (GUI must be running)
- Start a job (creates + queues by default):
  `kyco job start --file <path> --mode <mode_or_chain> --prompt "<what to do>" [--agent <id>] [--agents a,b] [--force-worktree]`
- Abort/stop a job:
  `kyco job abort <job_id>`
- Delete a job from the GUI list:
  `kyco job delete <job_id> [--cleanup-worktree]`
- Wait until done/failed/rejected/merged:
  `kyco job wait <job_id>`
- Fetch output:
  - Full output: `kyco job output <job_id>`
  - Summary only: `kyco job output <job_id> --summary`
  - State only: `kyco job output <job_id> --state`
- Inspect job JSON: `kyco job get <job_id> --json`
- Continue a session job with a follow-up prompt (creates a new job):
  `kyco job continue <job_id> --prompt "<follow-up>" [--pending]`

Mode CRUD (only with explicit user confirmation)
- Create/update mode: `kyco mode set <name> [--prompt ...] [--system-prompt ...] [--aliases ...] [--readonly] ...`
- Delete mode: `kyco mode delete <name>`

Batch job creation (efficient pattern for many files)
- Use ripgrep to find files, write to a temp file, then loop:
  ```bash
  # Example: Add tests to all .rs files in src/
  rg --files -g '*.rs' src/ > /tmp/files.txt
  while read file; do
    kyco job start --file "$file" --mode test --prompt "Add unit tests" --pending
  done < /tmp/files.txt
  ```
- Use --pending flag to create jobs without auto-queueing (review first in GUI)
- For multi-agent comparison: `--agents claude,codex` creates parallel jobs

Orchestration pattern
- Start a job, wait for completion, read its output/state, then decide follow-ups.
- If you start multiple jobs, keep track of IDs and report progress to the user.
- For batch operations: create all jobs first (--pending), let user review in GUI, then queue.
"#;

impl KycoApp {
    /// Launch the orchestrator in a new Terminal.app window
    pub(crate) fn launch_orchestrator(&mut self) -> anyhow::Result<()> {
        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!("Orchestrator launch is only supported on macOS right now.");
        }

        #[cfg(target_os = "macos")]
        {
            let kyco_dir = self.work_dir.join(".kyco");
            std::fs::create_dir_all(&kyco_dir)?;

            // Get orchestrator settings from config
            let (custom_cli, custom_prompt, default_agent) = self
                .config
                .read()
                .map(|cfg| {
                    let gui = &cfg.settings.gui;
                    (
                        gui.orchestrator.cli_command.trim().to_string(),
                        gui.orchestrator.system_prompt.trim().to_string(),
                        gui.default_agent.trim().to_lowercase(),
                    )
                })
                .unwrap_or_default();

            // Use custom prompt or fallback to built-in default
            let prompt = if custom_prompt.is_empty() {
                ORCHESTRATOR_SYSTEM_PROMPT.to_string()
            } else {
                custom_prompt
            };

            let prompt_file = kyco_dir.join("orchestrator_system_prompt.txt");
            std::fs::write(&prompt_file, &prompt)?;

            // Use custom CLI command or generate default based on agent
            let command = if !custom_cli.is_empty() {
                // Replace {prompt_file} placeholder with actual path
                custom_cli.replace("{prompt_file}", ".kyco/orchestrator_system_prompt.txt")
            } else {
                let agent = if default_agent.is_empty() {
                    "claude"
                } else {
                    default_agent.as_str()
                };
                match agent {
                    "codex" => {
                        "codex \"$(cat .kyco/orchestrator_system_prompt.txt)\"".to_string()
                    }
                    _ => {
                        "claude --append-system-prompt \"$(cat .kyco/orchestrator_system_prompt.txt)\""
                            .to_string()
                    }
                }
            };

            let session_id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let args = vec!["-lc".to_string(), command.clone()];
            TerminalSession::spawn(session_id, "bash", &args, "", &self.work_dir)?;

            self.logs.push(LogEvent::system(format!(
                "Orchestrator started in Terminal.app ({})",
                if custom_cli.is_empty() {
                    if default_agent.is_empty() {
                        "claude"
                    } else {
                        &default_agent
                    }
                } else {
                    "custom"
                }
            )));
            Ok(())
        }
    }
}
