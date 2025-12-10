<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

**Stay in control with AI coding.** Select code in your IDE, run targeted AI tasks, review diffs. No more endless agent sessions - just focused changes where you need them.

## Why KYCo?

Coding agents can spiral into hour-long sessions that touch half your codebase. KYCo takes a different approach:

- **Focused Changes**: Select specific code lines, run a mode, get targeted changes - not a full repo rewrite
- **Multi-Agent Power**: Run Claude, Codex, or Gemini in parallel with concurrent jobs and git worktree isolation
- **Voice-First Workflow**: Define tasks via Whisper speech-to-text - faster than typing prompts
- **You Stay in Control**: Review every diff, accept or reject changes, keep your codebase predictable

## Features

- **Native Desktop GUI**: Built with egui, runs as a standalone application
- **IDE Integration**: VS Code and JetBrains extensions send selections directly to KYCo
- **Batch Processing**: Process multiple files at once with the same mode
- **Grep Search**: Find files by pattern and process them as a batch
- **Multi-Agent Support**: Works with Claude, Codex, and Gemini CLI
- **Concurrent Jobs**: Run multiple AI tasks in parallel
- **Chains**: Automated multi-step workflows (review → fix → test)
- **Git Worktrees**: Optionally isolate changes in separate worktrees
- **Voice Input**: Trigger tasks with voice commands (experimental)
- **Cross-Platform**: macOS, Windows, and Linux

## How It Works

### Single Selection
1. **Select code** in your IDE (VS Code or JetBrains)
2. **Press the hotkey** (`Cmd+Alt+Y` / `Ctrl+Alt+Y`) to send the selection to KYCo
3. **Choose a mode** in the KYCo GUI (refactor, fix, test, etc.)
4. **Review the diff** when the AI completes the task
5. **Accept or reject** the changes

No comment markers required - just select and send!

### Batch Processing
Process multiple files at once with the same mode and prompt:

1. **Select files** in your IDE's file explorer (multi-select with Cmd/Ctrl+Click)
2. **Right-click** and choose "KYCo: Send Files (Batch)"
3. **Enter mode and prompt** in the KYCo batch popup (e.g., `refactor clean up formatting`)
4. **Jobs are created** for each file with the same configuration
5. **Review each diff** as jobs complete

**Use cases:**
- Refactor all files in a directory
- Add documentation to multiple modules
- Apply consistent style changes across files
- Add tests to multiple related files

### Grep Search & Send
Find files matching a pattern and process them as a batch:

1. **Press hotkey** (`Cmd+Alt+Shift+G` / `Ctrl+Alt+Shift+G`) or use Command Palette: "KYCo: Search & Send (Grep)"
2. **Enter search pattern** (regex supported, e.g., `TODO|FIXME`, `async function`, `#\[deprecated\]`)
3. **Enter file filter** (optional glob pattern, e.g., `**/*.ts`, `src/**/*.rs`)
4. **Confirm** the matching files to send to KYCo
5. **Enter mode and prompt** in the batch popup

**Example workflows:**
| Pattern | Use Case |
|---------|----------|
| `TODO\|FIXME` | Find and fix all TODO comments |
| `console\.log` | Remove debug logging |
| `any` | Fix TypeScript `any` types |
| `unsafe\s*\{` | Review unsafe Rust blocks |
| `@deprecated` | Migrate deprecated code |

## Installation

### Prerequisites

You need one of the supported AI CLIs installed:
- [Claude Code](https://claude.ai/code) (`claude`) - Recommended
- [Codex](https://github.com/openai/codex) (`codex`)
- [Gemini CLI](https://github.com/google/gemini-cli) (`gemini`)

### macOS

```bash
# Apple Silicon (M1/M2/M3/M4)
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64

# Intel Mac
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-x64

# Make executable and move to PATH
chmod +x kyco
sudo mv kyco /usr/local/bin/

# Remove macOS quarantine (if blocked by Gatekeeper)
xattr -d com.apple.quarantine /usr/local/bin/kyco
```

### Linux

```bash
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-linux-x64
chmod +x kyco
sudo mv kyco /usr/local/bin/
```

### Windows

Download `kyco-windows-x64.exe` from [Releases](https://github.com/MAF2414/kyco/releases/latest) and add to your PATH.

Or with PowerShell:
```powershell
Invoke-WebRequest -Uri "https://github.com/MAF2414/kyco/releases/latest/download/kyco-windows-x64.exe" -OutFile "kyco.exe"
Move-Item kyco.exe C:\Windows\System32\
```

### From Source

```bash
git clone https://github.com/MAF2414/kyco.git
cd kyco
cargo install --path .
```
Requires Rust 1.75+

### IDE Extensions

**VS Code:**
1. Download `kyco-vscode.vsix` from [Releases](https://github.com/MAF2414/kyco/releases/latest)
2. Install: `code --install-extension kyco-vscode.vsix`

**JetBrains (IntelliJ, WebStorm, PyCharm, etc.):**
1. Download `kyco-jetbrains.zip` from [Releases](https://github.com/MAF2414/kyco/releases/latest)
2. Settings → Plugins → ⚙️ → Install Plugin from Disk → Select the zip file

## Quick Start

1. **Initialize KYCo in your project:**
   ```bash
   kyco init
   ```
   This creates `.kyco/config.toml` with default configuration.

2. **Launch KYCo:**
   ```bash
   kyco
   ```

3. **In your IDE**, select some code and press `Cmd+Alt+Y` (Mac) or `Ctrl+Alt+Y` (Windows/Linux)

4. **In the KYCo GUI**, choose a mode and agent, then run the job

5. **Review the diff** and accept/reject changes

## Built-in Modes

| Mode | Aliases | Description |
|------|---------|-------------|
| `refactor` | `r`, `ref` | Improve code structure while preserving behavior |
| `tests` | `t`, `test` | Write comprehensive unit tests |
| `docs` | `d`, `doc` | Add documentation |
| `review` | `v`, `rev` | Analyze code for issues (read-only) |
| `fix` | `f` | Fix specific bugs with minimal changes |
| `implement` | `i`, `impl` | Implement new functionality (YAGNI-focused) |
| `optimize` | `o`, `opt` | Optimize for performance |
| `explain` | `e`, `exp` | Explain what code does (read-only) |
| `commit` | `cm`, `git` | Create git commits with conventional messages |
| `decouple` | `dec`, `di` | Introduce dependency injection |
| `extract` | `ex`, `split` | Extract code into reusable units |
| `logging` | `log`, `l` | Add meaningful logging (less is more) |
| `security` | `sec`, `harden` | Fix security vulnerabilities (OWASP Top 10) |
| `types` | `ty`, `typing` | Add type annotations |
| `coverage` | `cov` | Improve test coverage |
| `nullcheck` | `null`, `npe` | Find and fix null safety issues |
| `migrate` | `mig`, `upgrade` | Migrate to new APIs/versions |
| `cleanup` | `clean`, `tidy` | Remove dead code and cruft |

## Chains (Multi-Step Workflows)

Chains execute multiple modes in sequence with conditional triggers:

```toml
[chain.review-and-fix]
description = "Review code, fix issues, then test"
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
    { mode = "tests", trigger_on = ["fixed"] },
]
```

**Built-in chains:** `review-and-fix`, `implement-and-test`, `refactor-safe`, `secure-and-test`, `modernize`, `quality-gate`, and more.

## Configuration

Edit `.kyco/config.toml` to customize behavior:

```toml
[settings]
max_concurrent_jobs = 4      # Parallel job limit
auto_run = false             # Auto-start jobs
use_worktree = false         # Isolate jobs in git worktrees

[agent.claude]
aliases = ["c", "cl"]
binary = "claude"
allowed_tools = ["Read"]     # Restrict agent tools

[mode.refactor]
aliases = ["r", "ref"]
prompt = "..."
system_prompt = "..."
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
```

## Keyboard Shortcuts

### IDE Extension (VS Code / JetBrains)

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| Send Selection | `Cmd+Alt+Y` | `Ctrl+Alt+Y` |
| Grep & Send | `Cmd+Alt+Shift+G` | `Ctrl+Alt+Shift+G` |
| Batch (Context Menu) | Right-click in Explorer | Right-click in Explorer |

### KYCo GUI

| Action | Key |
|--------|-----|
| Tab completion | `Tab` |
| Execute job | `Enter` |
| Execute in worktree | `Shift+Enter` |
| Voice input | `Cmd+D` / `Ctrl+D` |
| Close popup | `Esc` |
| Navigate suggestions | `↑` / `↓` |
| Navigate jobs | `j` / `k` or `↑` / `↓` |
| Toggle auto-run | `Shift+A` |
| Toggle auto-scan | `Shift+S` |

## CLI Commands

```bash
kyco                    # Launch GUI (default)
kyco gui                # Launch GUI explicitly
kyco scan               # List all markers in codebase
kyco status             # Show job status
kyco init               # Create config file
kyco --help             # Show all options
```

## Architecture

```
src/
├── agent/      # AI agent integrations (Claude, Codex, Gemini)
├── cli/        # Command-line interface
├── config/     # Configuration management
├── domain/     # Core domain models
├── git/        # Git and worktree integration
├── gui/        # Desktop GUI (eframe/egui)
│   ├── jobs/       # Job list and management
│   ├── selection/  # IDE selection handling
│   ├── diff/       # Diff viewer
│   ├── voice/      # Voice input (experimental)
│   ├── modes/      # Mode configuration UI
│   ├── agents/     # Agent configuration UI
│   └── chains/     # Chain configuration UI
├── job/        # Job scheduling and execution
└── scanner/    # Codebase scanning
```

## Philosophy

KYCo is built on the belief that AI should augment, not replace, developer understanding. Every prompt is designed to:

1. **Explain changes** - The AI must say what it did and why
2. **Keep it minimal** - YAGNI: only implement what's requested
3. **Match existing patterns** - Follow the codebase's conventions
4. **Never surprise** - No hidden changes, everything in the diff

**Know your codebase. Don't just vibe with it.**

## License

[CC BY-NC-ND 4.0](LICENSE) - You may use and share this software, but commercial use and modifications require permission from the author.
