<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

**Stay in control with AI coding.** Select code in your IDE, run targeted AI tasks, review diffs. No more endless agent sessions - just focused changes where you need them.

## Why KYCo?

Coding agents can spiral into hour-long sessions that touch half your codebase. KYCo takes a different approach:

- **Focused Changes**: Select specific code lines, run a mode, get targeted changes
- **Multi-Agent Power**: Run Claude or Codex in parallel with concurrent jobs
- **Voice-First Workflow**: Define tasks via Whisper speech-to-text
- **You Stay in Control**: Review every diff, accept or reject changes

## Installation

### Prerequisites

- Node.js >= 18 (for the local SDK Bridge server)
- Claude CLI or Codex CLI installed

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

2. **Launch KYCo:**
   ```bash
   kyco
   ```

3. **In your IDE**, select some code and press `Cmd+Alt+Y` (Mac) or `Ctrl+Alt+Y` (Windows/Linux)

4. **Choose a mode** and agent, then run the job

5. **Review the diff** and accept/reject changes

## Built-in Modes

| Mode | Aliases | Description |
|------|---------|-------------|
| `chat` | `c` | Interactive conversation about the codebase |
| `implement` | `i`, `impl` | Implement new functionality |
| `review` | `r`, `rev` | Analyze code for issues (read-only) |
| `fix` | `f` | Fix specific bugs with minimal changes |
| `plan` | `p` | Create implementation plans (read-only) |

## Chains

Chains execute multiple modes in sequence:

```toml
[chain."review+fix"]
description = "Review code and fix any issues found"
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
]
```

## Configuration

Edit `.kyco/config.toml` to customize behavior:

```toml
[settings]
max_concurrent_jobs = 4      # Parallel job limit
auto_run = true              # Auto-start jobs
use_worktree = false         # Isolate jobs in git worktrees

[agent.claude]
aliases = ["c", "cl"]
sdk = "claude"

[agent.codex]
aliases = ["x", "cx"]
sdk = "codex"
```

## Keyboard Shortcuts

### IDE Extension

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| Send Selection | `Cmd+Alt+Y` | `Ctrl+Alt+Y` |
| Grep & Send | `Cmd+Alt+Shift+G` | `Ctrl+Alt+Shift+G` |

### KYCo GUI

| Action | Key |
|--------|-----|
| Execute job | `Enter` |
| Execute in worktree | `Shift+Enter` |
| Voice input | `Cmd+D` / `Ctrl+D` |
| Close popup | `Esc` |
| Navigate jobs | `j` / `k` or `↑` / `↓` |
| Toggle auto-run | `Shift+A` |

## CLI Commands

```bash
kyco                    # Launch GUI (default)
kyco init               # Create config file
kyco status             # Show job status
kyco job --help         # Start/inspect jobs via the running GUI (/ctl API)
kyco agent --help       # List/show configured agents
kyco chain --help       # List/show configured chains
kyco mode --help        # CRUD modes in .kyco/config.toml
kyco --help             # Show all options
```

## Orchestrator (External Agent)

KYCo can be used with an external orchestrator agent (Claude Code or Codex) that controls jobs via CLI commands.

1. Start KYCo GUI (so the user sees all jobs/results):
   ```bash
   kyco
   ```

   You can also use the **Orchestrator** button in the KYCo status bar to launch an external Claude/Codex session in Terminal.app (uses `settings.gui.default_agent`).

2. In a second terminal (same workspace), start your agent and let it call `kyco job ...`:
   - Create and queue a job:
     ```bash
     kyco job start --file src/foo.rs --mode refactor --prompt "Clean this up"
     ```
   - Wait for completion (event-like):
     ```bash
     kyco job wait 1
     ```
   - Abort a running job:
     ```bash
     kyco job abort 1
     ```
   - Continue a session job with a follow-up prompt (creates a new job in the same session/worktree):
     ```bash
     kyco job continue 1 --prompt "Please also update the tests"
     ```
   - Get the last response/output and pass it into a follow-up job:
     ```bash
     out="$(kyco job output 1)"
     kyco job start --file src/foo.rs --mode fix --prompt "$out"
     ```
   - Delete a job from the GUI list (optional worktree cleanup):
     ```bash
     kyco job delete 1 --cleanup-worktree
     ```

## License

[CC BY-NC-ND 4.0](LICENSE) - You may use and share this software, but commercial use and modifications require permission from the author.
