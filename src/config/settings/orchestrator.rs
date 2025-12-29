//! Orchestrator settings for external CLI sessions

use serde::{Deserialize, Serialize};

/// Orchestrator settings for external CLI sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorSettings {
    /// The CLI agent to use for the orchestrator.
    /// Options: "claude", "codex"
    /// Default: "claude"
    #[serde(default = "default_orchestrator_cli_agent")]
    pub cli_agent: String,

    /// The CLI command to use for the orchestrator.
    /// Use `{prompt_file}` as placeholder for the system prompt file path.
    /// Examples:
    /// - "claude --append-system-prompt \"$(cat {prompt_file})\""
    /// - "codex \"$(cat {prompt_file})\""
    /// - "aider --model gpt-4"
    /// If empty, auto-generates based on cli_agent
    #[serde(default)]
    pub cli_command: String,

    /// Custom system prompt for the orchestrator.
    /// If empty, uses the built-in default orchestrator prompt.
    #[serde(default = "default_orchestrator_system_prompt")]
    pub system_prompt: String,
}

fn default_orchestrator_cli_agent() -> String {
    "claude".to_string()
}

/// Default system prompt for the orchestrator
pub fn default_orchestrator_system_prompt() -> String {
    r#"You are an interactive KYCo Orchestrator running in the user's workspace.

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
- View job diff (shows changes made by the job):
  `kyco job diff <job_id> [--json]`
- Merge job changes into base branch:
  `kyco job merge <job_id> [-m "<commit message>"]`
- Reject job changes and cleanup worktree:
  `kyco job reject <job_id>`

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
- For batch operations: create all jobs first (--pending), let user review in GUI, then queue."#.to_string()
}

impl Default for OrchestratorSettings {
    fn default() -> Self {
        Self {
            cli_agent: default_orchestrator_cli_agent(),
            cli_command: String::new(),
            system_prompt: default_orchestrator_system_prompt(),
        }
    }
}
