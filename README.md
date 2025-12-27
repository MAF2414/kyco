<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

**Stay in control with AI coding.** Select code in your IDE, run targeted AI tasks, review diffs. No more endless agent sessions - just focused changes where you need them.

## Why KYCo?

Coding agents can spiral into hour-long sessions that touch half your codebase. KYCo takes a different approach:

- **Focused Changes**: Select specific code lines, run a mode, get targeted changes
- **Multi-Agent Power**: Run Claude or Codex in parallel with concurrent jobs
- **Voice-First Workflow**: Dictate tasks via Whisper speech-to-text - even into other apps
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
| `refactor` | `ref` | Improve code structure without changing behavior |
| `test` | `t` | Generate unit tests |
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

Built-in chains include:
- `refactor-safe` - Review → Refactor → Test
- `implement-and-test` - Implement → Test
- `quality-gate` - Review → Security → Types → Coverage

## Configuration

KYCo stores configuration in `~/.kyco/config.toml` (global) or `.kyco/config.toml` (per-project):

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

[mode.custom]
aliases = ["cu"]
prompt = "Your custom prompt here"
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
| Voice input (popup) | `Cmd+D` / `Ctrl+D` |
| Close popup | `Esc` |
| Navigate jobs | `j` / `k` or `↑` / `↓` |
| Toggle auto-run | `Shift+A` |

### Global Voice Hotkey

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| Voice dictation | `Cmd+Shift+V` | `Ctrl+Shift+V` |

Press once to start recording, press again to stop. The transcribed text is automatically pasted into the focused application - works with any app, including terminal-based tools like Claude Code.

## CLI Commands

```bash
kyco                    # Launch GUI (default)
kyco init               # Create config file
kyco status             # Show job status
kyco job --help         # Start/inspect jobs via the running GUI
kyco agent --help       # List/show configured agents
kyco chain --help       # List/show configured chains
kyco mode --help        # CRUD modes in config.toml
kyco --help             # Show all options
```

## Orchestrator (External Agent)

KYCo can be controlled by an external orchestrator agent (Claude Code or Codex) via CLI commands.

1. Start KYCo GUI (so the user sees all jobs/results):
   ```bash
   kyco
   ```

   Use the **Orchestrator** button in the status bar to launch an external agent session in Terminal.app.

2. In a second terminal (same workspace), the agent can control jobs:
   ```bash
   # Create and queue a job
   kyco job start --file src/foo.rs --mode refactor --prompt "Clean this up"

   # Wait for completion
   kyco job wait 1

   # Get output for follow-up
   out="$(kyco job output 1)"
   kyco job start --file src/foo.rs --mode fix --prompt "$out"

   # Continue a session with follow-up prompt
   kyco job continue 1 --prompt "Please also update the tests"

   # Abort or delete jobs
   kyco job abort 1
   kyco job delete 1 --cleanup-worktree
   ```

## Voice Input

KYCo uses Whisper for speech-to-text transcription. Voice dependencies are auto-installed on first use.

**In the selection popup:**
- Click the microphone button or press `Cmd+D` / `Ctrl+D`
- Speak your mode and prompt (e.g., "refactor this function")
- Press Enter to execute

**Global dictation (any app):**
- Press `Cmd+Shift+V` / `Ctrl+Shift+V` from any application
- Speak your text
- Press the hotkey again to stop - text is auto-pasted

## Support

If you find KYCo useful, consider [sponsoring the project](https://github.com/sponsors/MAF2414).

## License

[Business Source License 1.1](LICENSE)

You may use the Licensed Work for any purpose, including production use, as long as you do not offer it as a hosted service or sell it as a competing product.

On 2029-01-01, the license converts to Apache License 2.0.

For commercial licensing inquiries, contact via [GitHub](https://github.com/MAF2414/kyco).
