<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

**One agent, one task, one file.** That's the idea.

Instead of letting a single agent loose on your entire codebase for hours, KYCo spawns focused agents that each handle exactly one task on a specific code selection. The result: faster iterations, lower hallucination rates, and changes you can actually review.

## How it works

1. Select code in your IDE
2. Tell KYCo what to do (refactor, fix, test, etc.)
3. A dedicated agent works on just that
4. Review the diff, accept or reject

You can also orchestrate multiple jobs from coding CLIs like Claude Code or Codex - KYCo handles the job queue while you stay in control of what gets merged.

## Why this approach?

- **Focused scope** - Each agent sees only what it needs. Less context = fewer hallucinations.
- **Parallel execution** - Run multiple agents on different files simultaneously
- **Reviewable changes** - Every job produces a diff you can inspect before merging
- **CLI orchestration** - External agents can spawn and manage KYCo jobs programmatically

## Installation

### Prerequisites

- Node.js >= 18 (for the SDK Bridge server)
- Claude CLI or Codex CLI

### macOS

```bash
# Apple Silicon
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64

# Intel
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-x64

chmod +x kyco
sudo mv kyco /usr/local/bin/

# If Gatekeeper blocks it:
xattr -d com.apple.quarantine /usr/local/bin/kyco
```

### Linux

```bash
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-linux-x64
chmod +x kyco
sudo mv kyco /usr/local/bin/
```

### Windows

Download `kyco-windows-x64.exe` from [Releases](https://github.com/MAF2414/kyco/releases/latest) and add to PATH.

### From source

```bash
git clone https://github.com/MAF2414/kyco.git
cd kyco
cargo install --path .
```

Requires Rust 1.75+

### IDE Extensions

**VS Code:**
```bash
code --install-extension kyco-vscode.vsix
```

**JetBrains:** Settings → Plugins → Install from Disk → select the zip

## Quick start

```bash
kyco init    # create config
kyco         # start GUI
```

In your IDE: select code → `Cmd+Alt+Y` (Mac) or `Ctrl+Alt+Y` (Win/Linux) → pick mode → review diff

## Modes

| Mode | Alias | Description |
|------|-------|-------------|
| `chat` | `c` | Discuss the selected code |
| `implement` | `i` | Add new functionality |
| `review` | `r` | Analyze for issues (read-only) |
| `fix` | `f` | Fix a specific bug |
| `refactor` | `ref` | Improve structure, keep behavior |
| `test` | `t` | Generate tests |
| `plan` | `p` | Create implementation plan (read-only) |

## Chains

Run multiple modes in sequence:

```toml
[chain."review+fix"]
description = "Review first, then fix issues"
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
]
```

Built-in chains: `refactor-safe`, `implement-and-test`, `quality-gate`

## Configuration

Config lives in `~/.kyco/config.toml` (global) or `.kyco/config.toml` (per project):

```toml
[settings]
max_concurrent_jobs = 4
auto_run = true
use_worktree = false    # isolate jobs in git worktrees

[agent.claude]
aliases = ["c", "cl"]
sdk = "claude"

[agent.codex]
aliases = ["x", "cx"]
sdk = "codex"

[mode.custom]
aliases = ["cu"]
prompt = "Your custom instruction here"
```

## Keyboard shortcuts

### IDE

| Action | Mac | Win/Linux |
|--------|-----|-----------|
| Send selection | `Cmd+Alt+Y` | `Ctrl+Alt+Y` |
| Grep & send | `Cmd+Alt+Shift+G` | `Ctrl+Alt+Shift+G` |

### KYCo GUI

| Action | Key |
|--------|-----|
| Execute | `Enter` |
| Execute in worktree | `Shift+Enter` |
| Voice input | `Cmd+D` / `Ctrl+D` |
| Close popup | `Esc` |
| Navigate jobs | `j`/`k` or arrows |

### Global voice hotkey

| Action | Mac | Win/Linux |
|--------|-----|-----------|
| Dictation | `Cmd+Shift+V` | `Ctrl+Shift+V` |

Works from any app, including terminal-based tools. Press to start recording, press again to stop - text gets pasted into the focused application.

## CLI

```bash
kyco                    # start GUI
kyco init               # create config
kyco status             # show jobs

# Job management (useful for orchestration)
kyco job start --file src/foo.rs --mode fix --prompt "Fix the null check"
kyco job wait 1
kyco job output 1
kyco job continue 1 --prompt "Add tests for this"
kyco job abort 1
```

## Orchestrator mode

KYCo can be controlled by an external coding agent:

1. Start `kyco` (GUI shows all jobs)
2. In another terminal, run your orchestrator agent (Claude Code, Codex, etc.)
3. The orchestrator spawns focused jobs via `kyco job start ...`
4. You review results in the KYCo GUI

This pattern lets you combine high-level planning (orchestrator) with focused execution (KYCo jobs) while keeping humans in the loop for code review.

## Voice input

KYCo uses Whisper for speech-to-text. Dependencies install automatically on first use.

- **In popup:** `Cmd+D` → speak → `Enter`
- **Global:** `Cmd+Shift+V` → speak → `Cmd+Shift+V` → auto-paste

## License

[Business Source License 1.1](LICENSE)

Free for any use, including production - just don't offer it as a hosted service or competing product. Converts to Apache 2.0 on 2029-01-01.

## Support

If KYCo improves your workflow, consider [sponsoring](https://github.com/sponsors/MAF2414) the project.

Questions or feedback: [GitHub Issues](https://github.com/MAF2414/kyco/issues)
