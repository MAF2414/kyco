<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

Hey! 👋 You know how it goes - you fire up an AI agent and 2 hours later it's rewritten half your codebase. KYCo does things differently:

**Select code → tell it what to do → review the diff → done.**

No more endless agent sessions. You stay in control.

## What's this thing do?

- 🎯 **Targeted changes** - Select code, pick a mode, get exactly the change you need
- 🤖 **Multi-agent** - Run Claude and Codex in parallel? No problem
- 🎤 **Voice input** - Just speak what you want (Whisper handles transcription)
- 👀 **You decide** - Every diff is shown to you, you say accept or reject

## Installation

### What you need

- Node.js >= 18 (for the SDK Bridge server)
- Claude CLI or Codex CLI

### macOS

```bash
# M1/M2/M3/M4
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64

# Intel
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-x64

chmod +x kyco
sudo mv kyco /usr/local/bin/

# If Gatekeeper complains:
xattr -d com.apple.quarantine /usr/local/bin/kyco
```

### Linux

```bash
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-linux-x64
chmod +x kyco
sudo mv kyco /usr/local/bin/
```

### Windows

Grab `kyco-windows-x64.exe` from [Releases](https://github.com/MAF2414/kyco/releases/latest) and add it to your PATH.

### Build it yourself

```bash
git clone https://github.com/MAF2414/kyco.git
cd kyco
cargo install --path .
```

Needs Rust 1.75+

### IDE Extensions

**VS Code:**
```bash
# grab the vsix from Releases, then:
code --install-extension kyco-vscode.vsix
```

**JetBrains:** Settings → Plugins → ⚙️ → Install from Disk → pick the zip

## Let's go

```bash
kyco init    # create config
kyco         # start GUI
```

Then in your IDE: select code → `Cmd+Alt+Y` (Mac) or `Ctrl+Alt+Y` → pick a mode → Enter → review diff → Done ✅

## The main modes

| Mode | Shortcut | What's it do? |
|------|----------|---------------|
| `chat` | `c` | Just chat about the code |
| `implement` | `i` | Build new features |
| `review` | `r` | Check code for issues (read-only) |
| `fix` | `f` | Fix bugs |
| `refactor` | `ref` | Clean up without changing behavior |
| `test` | `t` | Write tests |
| `plan` | `p` | Create a plan (read-only) |

## Chains - multiple modes in sequence

Want to review first, then fix? That's what chains are for:

```toml
[chain."review+fix"]
description = "Check first, then fix"
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
]
```

Built-in chains:
- `refactor-safe` → Review → Refactor → Test
- `implement-and-test` → Implement → Test
- `quality-gate` → Review → Security → Types → Coverage

## Config

Lives in `~/.kyco/config.toml` (global) or `.kyco/config.toml` (per project):

```toml
[settings]
max_concurrent_jobs = 4      # how many jobs in parallel
auto_run = true              # start jobs immediately
use_worktree = false         # isolate jobs in git worktrees

[agent.claude]
aliases = ["c", "cl"]
sdk = "claude"

[agent.codex]
aliases = ["x", "cx"]
sdk = "codex"

# define your own modes:
[mode.cleanup]
aliases = ["cu"]
prompt = "Clean up this code, remove dead code"
```

## Shortcuts

### In your IDE

| What | Mac | Windows/Linux |
|------|-----|---------------|
| Send selection | `Cmd+Alt+Y` | `Ctrl+Alt+Y` |
| Grep & send | `Cmd+Alt+Shift+G` | `Ctrl+Alt+Shift+G` |

### In KYCo

| What | Key |
|------|-----|
| Start job | `Enter` |
| With worktree | `Shift+Enter` |
| Voice | `Cmd+D` |
| Close popup | `Esc` |
| Navigate jobs | `j`/`k` or arrow keys |
| Toggle auto-run | `Shift+A` |

### Voice hotkey (works anywhere!)

| What | Mac | Windows/Linux |
|------|-----|---------------|
| Start/stop dictation | `Cmd+Shift+V` | `Ctrl+Shift+V` |

The cool part: works from anywhere! Even when Claude Code is running in the terminal. Press once = recording, press again = done, text gets auto-pasted.

## CLI Commands

```bash
kyco                    # start GUI
kyco init               # create config
kyco status             # show jobs
kyco job start --file src/foo.rs --mode fix --prompt "Fix the bug"
kyco job wait 1         # wait for job to finish
kyco job output 1       # get output
kyco job continue 1 --prompt "Add tests too"
kyco job abort 1        # cancel job
```

## Orchestrator Mode

You can have an external agent (Claude Code / Codex) control KYCo:

1. Start KYCo GUI (so you can see everything)
2. In another terminal, start your agent
3. The agent calls `kyco job ...` commands

There's also an Orchestrator button in the status bar that launches a Claude/Codex session in Terminal.app.

## Voice Input

KYCo uses Whisper for speech-to-text. Dependencies get installed automatically on first use.

**In the popup:** Click the mic button or press `Cmd+D` → speak → Enter

**Global (any app):** `Cmd+Shift+V` → speak → press again → text gets inserted

## Support

If KYCo helps you out, consider becoming a [sponsor](https://github.com/sponsors/MAF2414) ☕

## License

[Business Source License 1.1](LICENSE)

Free to use, even in production - as long as you don't offer it as a hosted service or sell it as a competing product. Becomes Apache 2.0 in 2029.

Questions? [GitHub Issues](https://github.com/MAF2414/kyco/issues) or just reach out 👋
