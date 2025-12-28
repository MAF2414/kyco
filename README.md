<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

**Point agents exactly where to look.**

When you tell a coding CLI "check our services for security issues", it might scan 5-10 files before the context window fills up. Half your code never gets looked at. KYCo flips this: you select the files, KYCo spawns a dedicated agent for each one. Every file gets full attention.

## The problem with general-purpose agents

Coding CLIs like Claude Code or Codex are generalists. They're powerful, but when you ask them to refactor a module or audit for vulnerabilities, they have to decide what to look at. Large codebases exceed context limits. Files get skipped. You hope for the best.

## How KYCo fixes this

1. **You define the scope** - Select specific files or lines in your IDE, or batch-select an entire folder
2. **One agent per target** - Each job gets a dedicated agent focused on exactly that code
3. **Parallel execution** - Run 4, 8, or more agents simultaneously across your codebase
4. **Full agent power** - Every KYCo job runs a complete Claude/Codex session with all tools available

The agents aren't limited - they have full repo access. But you're telling them exactly where to focus: "Look HERE. Do THIS." No hoping the generalist finds everything.

## System prompts for precision

Each mode can append or override the agent's system prompt. Instead of relying on generic instructions, you define exactly what you want:

```toml
[mode.security-audit]
prompt = "Analyze this code for security vulnerabilities. Check for injection, auth bypass, data exposure."
system_prompt = "You are a security auditor. Be thorough. Flag anything suspicious."
```

## Orchestrator for CLI power

Want the flexibility of a coding CLI but with focused execution? Use the orchestrator pattern:

1. Start KYCo GUI (shows all jobs, handles review)
2. Run Claude Code or Codex as your orchestrator
3. The orchestrator spawns KYCo jobs: `kyco job start --file src/auth.rs --mode security-audit`
4. It can wait for results, spawn follow-ups, or review changes itself
5. You make the final call on what gets merged

This gives you CLI-level autonomy for planning while ensuring every file gets dedicated attention.

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
| `rustloc300` | `rs300` | Reduce a Rust file to ≤300 LOC without behavior changes |
| `pythonloc300` | `py300` | Reduce a Python file to ≤300 LOC without behavior changes |
| `csharploc300` | `cs300` | Reduce a C# file to ≤300 LOC without behavior changes |
| `typescriptloc300` | `ts300` | Reduce a TypeScript file to ≤300 LOC without behavior changes |
| `kotlinloc300` | `kt300` | Reduce a Kotlin file to ≤300 LOC without behavior changes |
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
prompt = "Your instruction here"
system_prompt = "Optional: override or extend the agent's system prompt"
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

# Job management
kyco job start --file src/foo.rs --mode fix --prompt "Fix the null check"
kyco job wait 1
kyco job output 1
kyco job continue 1 --prompt "Add tests for this"
kyco job abort 1
```

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
