# KYCo - Know Your Codebase

**The antidote to vibe coding.** KYCo is a TUI orchestrator that lets you trigger AI coding tasks directly from comments in your code - with full transparency about what the AI does and why.

## Why KYCo?

In the age of "vibe coding" where developers blindly accept AI-generated code, KYCo takes a different approach:

- **Transparency**: Every mode requires the AI to explain what it changed and why
- **Control**: You define tasks with simple comment markers
- **Understanding**: Stay in sync with your codebase, even when AI helps

## Installation

### From Source (Rust)

```bash
# Clone the repository
git clone https://github.com/yourusername/kyco.git
cd kyco

# Build and install
cargo install --path .
```

### Prerequisites

- Rust 1.75+ (for building)
- One of the supported AI CLIs:
  - [Claude Code](https://claude.ai/code) (`claude`)
  - [Codex](https://github.com/openai/codex) (`codex`)
  - [Gemini CLI](https://github.com/google/gemini-cli) (`gemini`)

## Quick Start

1. **Initialize KYCo in your project:**
   ```bash
   kyco init
   ```
   This creates `.kyco/config.toml` with default configuration.

2. **Add markers to your code:**
   ```rust
   // @@refactor simplify this function
   fn complex_function() {
       // ...
   }

   // @@docs add documentation
   pub struct MyStruct {
       // ...
   }

   // @@fix handle the edge case when input is empty
   fn process(input: &str) {
       // ...
   }
   ```

3. **Run KYCo:**
   ```bash
   kyco run
   ```

4. **Watch the TUI** as jobs are discovered and executed. Review changes before applying.

## Marker Syntax

```
@@{agent:}?{mode} {description}?
```

Examples:
- `@@docs` - Add documentation (default agent: claude)
- `@@fix handle edge case` - Fix with description
- `@@claude:refactor` - Explicit agent
- `@@x:test add unit tests` - Codex with short alias

## Built-in Modes

| Mode | Aliases | Description |
|------|---------|-------------|
| `refactor` | `r`, `ref` | Improve code quality while preserving behavior |
| `tests` | `t`, `test` | Write comprehensive unit tests |
| `docs` | `d`, `doc` | Write documentation |
| `review` | `v`, `rev` | Analyze code (read-only, no changes) |
| `fix` | `f` | Fix specific bugs or issues |
| `implement` | `i`, `impl` | Implement new functionality |
| `optimize` | `o`, `opt` | Optimize for performance |
| `commit` | `cm`, `git` | Create git commits with conventional messages |

## Configuration

Edit `.kyco/config.toml` to customize:

```toml
[settings]
max_concurrent_jobs = 4      # Parallel job limit
debounce_ms = 500            # File watcher debounce
auto_run = false             # Auto-start jobs when found
marker_prefix = "@@"         # Comment marker prefix
use_worktree = false         # Isolate jobs in git worktrees

[agent.claude]
aliases = ["c", "cl"]
binary = "claude"
# ... more agent config

[mode.refactor]
aliases = ["r", "ref"]
prompt = "..."
system_prompt = "..."
```

## CLI Commands

```bash
kyco                    # Run TUI (default)
kyco run                # Run TUI with options
kyco scan               # List all markers in codebase
kyco status             # Show job status
kyco init               # Create config file
kyco --help             # Show all options
```

## How It Works

1. **Scan**: KYCo watches your codebase for marker comments
2. **Parse**: Markers are parsed into jobs with mode, agent, and description
3. **Execute**: Jobs run via the configured AI CLI (claude, codex, gemini)
4. **Explain**: The AI explains what it changed (transparency!)
5. **Review**: You review and accept/reject changes in the TUI

## Philosophy

KYCo is built on the belief that AI should augment, not replace, developer understanding. Every prompt is designed to:

1. Make the AI explain its reasoning
2. Keep changes minimal and focused
3. Follow existing code patterns
4. Never surprise the developer

**Know your codebase. Don't just vibe with it.**

## License

MIT
