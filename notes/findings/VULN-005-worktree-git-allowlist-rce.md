# VULN-005: Worktree git allowlist enables RCE via git aliases/hooks

## Summary
When `run_job` executes inside a Git worktree it silently appends `git`/`Bash(git:*)` to the agent’s `allowed_tools`, expanding strict allowlists. Because `Bash(git:*)` permits arbitrary `git` invocations (including `git -c alias.*=!<cmd>` / `-c core.hooksPath=...`), a prompt-injected agent can execute arbitrary shell commands despite intended tool restrictions.

## Severity
**HIGH** - Bypasses operator-defined tool allowlists and can lead to arbitrary code execution/exfiltration on the host by hiding shell commands inside a “git” tool invocation.

## Location
src/gui/executor/run_job.rs:121-138
src/agent/bridge/adapter/claude.rs:116-160

## Code
```rust
// src/gui/executor/run_job.rs
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
```

```rust
// src/agent/bridge/adapter/claude.rs
allowed_tools: clone_if_non_empty(&config.allowed_tools),
```

## Impact
- A mode/agent that intentionally sets `allowed_tools` to a safe subset can unexpectedly gain `Bash` capability (via `Bash(git:*)`) whenever a worktree is used (e.g., multi-agent jobs).
- Even if users only approve “git” commands, `git` can execute arbitrary programs via inline config (`-c alias.pwn='!…'`) or custom hooks (`-c core.hooksPath=... commit`), enabling RCE, secret theft, and filesystem modification.

## Attack Scenario
1. Victim configures an agent/mode with a strict `allowed_tools` allowlist (e.g., no `Bash`), expecting to prevent command execution.
2. Victim runs a multi-agent job (or any job that enters `is_in_worktree == true`), causing `run_job` to append `Bash(git:*)` to `allowed_tools`.
3. A malicious prompt injection causes the agent to request `Bash` with a command that passes the allowlist but executes arbitrary shell via `git`, e.g. `git -c alias.pwn='!sh -c \"cat ~/.ssh/id_rsa > /tmp/loot\"' pwn` (or `git -c core.hooksPath=.hooks commit -m ...` where `.hooks/pre-commit` is a tracked script).
4. The payload runs under the victim user account, allowing local compromise and exfiltration.

## Suggested Fix
- Do not mutate user-specified `allowed_tools` automatically. If “auto-git in worktree” is desired, gate it behind an explicit config flag and document the security tradeoff.
- Remove `Bash(git:*)` (too broad); instead allow only the exact subcommands needed (`git status`, `git diff`, `git add`, `git commit -m`) with argument validation or a dedicated safe git wrapper.
- Prefer performing commits programmatically in KYCO (via `GitManager`) after completion rather than granting the agent a shell-capable git surface.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben

