# KYCo - Know Your Codebase

**The antidote to vibe coding.** KYCo is a desktop application that lets you trigger AI coding tasks directly from your IDE - with full transparency about what the AI does and why.

## Why KYCo?

In the age of "vibe coding" where developers blindly accept AI-generated code, KYCo takes a different approach:

- **Transparency**: Every mode requires the AI to explain what it changed and why
- **Control**: You trigger tasks from your IDE, review diffs, accept or reject
- **Understanding**: Stay in sync with your codebase, even when AI helps

## Features

- **Native Desktop GUI**: Built with egui, runs as a standalone application
- **IDE Integration**: VS Code and JetBrains extensions send selections directly to KYCo
- **Multi-Agent Support**: Works with Claude, Codex, and Gemini CLI
- **Concurrent Jobs**: Run multiple AI tasks in parallel
- **Chains**: Automated multi-step workflows (review → fix → test)
- **Git Worktrees**: Optionally isolate changes in separate worktrees
- **Voice Input**: Trigger tasks with voice commands (experimental)
- **Cross-Platform**: macOS, Windows, and Linux

## How It Works

1. **Select code** in your IDE (VS Code or JetBrains)
2. **Press the hotkey** (`Cmd+Alt+Y` / `Ctrl+Alt+Y`) to send the selection to KYCo
3. **Choose a mode** in the KYCo GUI (refactor, fix, test, etc.)
4. **Review the diff** when the AI completes the task
5. **Accept or reject** the changes

No comment markers required - just select and send!

## Installation

### From Source

```bash
git clone https://github.com/MAF2414/kyco.git
cd kyco
cargo install --path .
```

### Prerequisites

- Rust 1.75+ (for building)
- One of the supported AI CLIs:
  - [Claude Code](https://claude.ai/code) (`claude`)
  - [Codex](https://github.com/openai/codex) (`codex`)
  - [Gemini CLI](https://github.com/google/gemini-cli) (`gemini`)

### IDE Extensions

**VS Code:**
```bash
cd vscode-extension
npm install && npm run compile
# Install the .vsix file
```

**JetBrains (IntelliJ, WebStorm, PyCharm, etc.):**
- Install from `jetbrains-plugin/` or JetBrains Marketplace

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

MIT
