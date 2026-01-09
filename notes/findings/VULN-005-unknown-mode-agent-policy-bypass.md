# VULN-005: Unknown mode/agent can bypass configured tool restrictions

## Summary
`Config::get_agent_for_mode()` and `Config::get_agent_for_job()` effectively “fail open” when given an unknown mode and/or agent ID. Combined with `/ctl/jobs` accepting arbitrary `mode`/`agent` strings, an attacker can bypass mode/agent tool restrictions and run jobs with permissive defaults.

## Severity
**HIGH** - A caller that can choose `mode`/`agent` (e.g., `/ctl/jobs` when auth is disabled, or any local process with the token) can bypass “read-only”/restricted modes and run jobs under a broadly-permissive agent configuration, enabling unintended file writes and (depending on backend) command execution.

## Location
src/config/lookup.rs:79-163
src/gui/executor/run_job.rs:117-120
src/gui/http_server/handlers/job_create.rs:54-80

## Code
```rust
// src/config/lookup.rs
pub fn get_agent_for_mode(&self, mode: &str) -> Cow<'_, str> {
    self.mode
        .get(mode)
        .and_then(|m| m.agent.as_deref())
        .map(Cow::Borrowed)
        .unwrap_or(Cow::Borrowed("claude"))
}

pub fn get_agent_for_job(&self, agent_id: &str, mode: &str) -> Option<AgentConfig> {
    let mut agent_config = self.get_agent(agent_id)?;
    if let Some(mode_config) = self.mode.get(mode) {
        /* apply allowed/disallowed tools only if mode exists */
        // ...
    }
    Some(agent_config)
}
```

```rust
// src/gui/executor/run_job.rs
let mut agent_config = config
    .get_agent_for_job(&job.agent_id, &job.mode)
    .unwrap_or_default();
```

```rust
// src/gui/http_server/handlers/job_create.rs
let mode = req.mode.trim();
if mode.is_empty() { /* ... */ }

// accepts arbitrary agent IDs without validation
if agents.is_empty() {
    let agent = req.agent.as_deref().unwrap_or("claude").trim().to_string();
    agents.push(agent);
}
```

## Impact
- Bypass “read-only” modes (e.g., modes that disallow `Write`/`Edit`) by submitting an unknown `mode` so no mode restrictions are applied.
- Bypass agent-specific tool allow/deny configuration by submitting an unknown `agent` so execution falls back to `AgentConfig::default()` (which uses empty allow/deny lists; empty `allowed_tools` is treated as “allow all” in this codebase).
- In environments where `/ctl/*` auth is disabled or the token is accessible, this enables unintended file modifications and potentially command execution through the agent backend.

## Attack Scenario
1. Victim runs the KYCo GUI with `/ctl/*` auth disabled (or attacker is a local process with the token) and relies on restricted modes/agents (e.g., `mode.review` is read-only).
2. Attacker sends `POST /ctl/jobs` with `mode` set to a non-existent string (and/or `agent` set to a non-existent ID).
3. The job is created and executed; `get_agent_for_job()` applies no mode restrictions for an unknown mode and `unwrap_or_default()` supplies a permissive fallback config if the agent lookup fails.
4. The agent runs without the intended tool restrictions, allowing edits/commands that were meant to be blocked.

## Suggested Fix
- Treat unknown `mode`/`agent` as an error at the boundary (e.g., validate in `handle_control_job_create()` and/or when queuing jobs).
- Avoid `unwrap_or_default()` for agent lookup; fail the job with a clear error when `get_agent_for_job()` returns `None`.
- If a fallback must exist, make it least-privileged (e.g., disallow `Write`/`Edit`/`Bash`, and set Codex sandbox to `read-only`) and log loudly.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben
