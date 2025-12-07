//! Init command implementation

use anyhow::{bail, Result};
use std::path::Path;

/// Default configuration content for kyco init
pub const DEFAULT_CONFIG: &str = r#"# KYCo Configuration - Know Your Codebase
# =======================
#
# Syntax: @@{agent:}?mode {description}?
#
# Examples:
#   // @@docs                      - Add documentation (default agent: claude)
#   // @@fix handle edge case      - Fix with description
#   // @@claude:refactor           - Explicit agent
#   // @@x:t add unit tests        - Codex tests (short aliases)
#   # @@review check for bugs      - Python comment

# ============================================================================
# SETTINGS - Global configuration options
# ============================================================================
#
# Available options:
#   max_concurrent_jobs - Maximum number of jobs to run simultaneously (default: 4)
#   debounce_ms         - Delay before scanning after file changes (default: 500)
#   auto_run            - Automatically start jobs when found (default: false)
#   marker_prefix       - The marker prefix to look for (default: "@@")
#   scan_exclude        - Glob patterns to exclude from scanning (default: [])
#   use_worktree        - Run jobs in isolated Git worktrees (default: false)

[settings]
max_concurrent_jobs = 4
debounce_ms = 500
auto_run = false
marker_prefix = "@@"
scan_exclude = []
use_worktree = false

# ============================================================================
# AGENTS - The AI backends that execute jobs
# ============================================================================
#
# Each agent defines how to invoke an AI CLI tool.
#
# Available options:
#   aliases             - Short names to reference this agent (e.g., "c" for "claude")
#   cli_type            - The CLI type: "claude", "codex", "gemini", or "custom"
#   binary              - The executable to run (e.g., "claude", "codex")
#   mode                - Execution mode: "print" (non-interactive) or "repl" (Terminal.app)
#   print_mode_args     - Arguments for print mode (e.g., ["-p"])
#   output_format_args  - Arguments for output format (e.g., ["--output-format", "stream-json"])
#   repl_mode_args      - Arguments for REPL/interactive mode
#   system_prompt_mode  - How to pass system prompts: "append", "replace", or "configoverride"
#   disallowed_tools    - Tools the agent cannot use (e.g., ["Write", "Edit"] for read-only)
#   allowed_tools       - Limit agent to only these tools
#   env                 - Environment variables to set when running the agent

[agent.claude]
aliases = ["c", "cl"]
cli_type = "claude"
binary = "claude"
print_mode_args = ["-p", "--allowedTools", "Read"]
output_format_args = ["--output-format", "stream-json", "--verbose"]
system_prompt_mode = "append"
allowed_tools = ["Read"]

[agent.codex]
aliases = ["x", "cx"]
cli_type = "codex"
binary = "codex"
print_mode_args = ["exec", "--sandbox", "workspace-write"]
output_format_args = ["--json"]
system_prompt_mode = "configoverride"
allowed_tools = ["Read"]

[agent.gemini]
aliases = ["g", "gm"]
cli_type = "gemini"
binary = "gemini"
system_prompt_mode = "replace"
allowed_tools = ["Read"]

# REPL mode agents - run in a separate Terminal.app window (macOS)
# Use "cr" for Claude REPL, "xr" for Codex REPL, etc.
[agent.cr]
cli_type = "claude"
binary = "claude"
mode = "repl"
repl_mode_args = ["--allowedTools", "Read"]
allowed_tools = ["Read"]

[agent.xr]
cli_type = "codex"
binary = "codex"
mode = "repl"

# ============================================================================
# MODES - Prompt templates that define HOW to instruct the agent
# ============================================================================
#
# Modes define the prompt template and system prompt for different task types.
#
# Available options:
#   aliases             - Short names for this mode (e.g., "r" for "refactor")
#   prompt              - The prompt template sent to the agent
#   system_prompt       - Additional context/instructions for the agent
#   agent               - Default agent for this mode (optional, defaults to "claude")
#   disallowed_tools    - Tools not allowed for this mode (e.g., ["Write", "Edit"] for review)
#   allowed_tools       - Limit to only these tools for this mode
#
# Prompt template placeholders:
#   {file}        - The source file path (e.g., "./src/main.rs")
#   {line}        - The start line number (e.g., "42")
#   {target}      - The target location with line range (e.g., "./src/main.rs:42-50")
#   {mode}        - The mode name (e.g., "refactor", "docs")
#   {description} - The user's description from the marker comment
#   {scope_type}  - The scope type (e.g., "file", "function", "line")
#   {ide_context} - Rich context from IDE (dependencies, related tests, etc.)
#
# When using the IDE extension (VS Code/JetBrains), additional context is provided:
#   - File and line selection
#   - Dependencies (files that import/use the selected code)
#   - Related tests (test files for the selected code)

[mode.refactor]
aliases = ["r", "ref"]
output_states = ["refactored"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Refactor `{target}`: {description}

{ide_context}

1. Read and understand the code
2. Check dependencies to avoid breaking changes
3. Refactor for clarity while preserving exact behavior
4. Set state to "refactored"
"""
system_prompt = """
You refactor code. Preserve exact behavior. Match project style.

DO:
- Improve naming, structure, readability
- Extract duplicated logic
- Simplify complex conditionals
- Check listed dependencies before changing signatures

DON'T:
- Change public APIs
- Add features or fix bugs
- Over-engineer
"""

[mode.tests]
aliases = ["t", "test"]
output_states = ["tests_pass", "tests_fail"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Write tests for `{target}`: {description}

{ide_context}

1. Check related tests for existing patterns
2. Write tests covering happy path, edge cases, and errors
3. Run tests and set state to "tests_pass" or "tests_fail"
"""
system_prompt = """
You write tests. Use the project's existing test framework and patterns.

COVER:
- Happy path (normal inputs)
- Edge cases (empty, boundary, null)
- Error cases (invalid inputs, exceptions)

DO:
- Check related tests first for style/framework
- One assertion focus per test
- Descriptive test names

DON'T:
- Test implementation details
- Depend on external services without mocking
"""

[mode.docs]
aliases = ["d", "doc"]
output_states = ["documented"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Document `{target}`: {description}

{ide_context}

1. Read the code and identify existing doc style
2. Write clear documentation with examples
3. Set state to "documented"
"""
system_prompt = """
You write documentation. Match the project's existing doc format.

INCLUDE:
- Purpose (what and why)
- Parameters (types, constraints, defaults)
- Returns (types, possible values)
- Examples for non-trivial code

DON'T:
- Over-document obvious code
- Include implementation details that may change
"""

[mode.review]
aliases = ["v", "rev"]
output_states = ["issues_found", "no_issues"]
prompt = """
Review `{target}`: {description}

{ide_context}

1. Read the code and its dependencies
2. Identify bugs, security issues, performance problems
3. Output findings with SEVERITY, LOCATION, ISSUE, SUGGESTION
4. Set state to "issues_found" or "no_issues"
"""
system_prompt = """
You review code. READ-ONLY - no edits.

CHECK FOR:
- Bugs: logic errors, null handling, race conditions
- Security: injection, auth issues, data exposure
- Performance: N+1 queries, memory leaks, missing caching
- Maintainability: complexity, unclear naming, missing error handling

OUTPUT FORMAT (per issue):
- SEVERITY: Critical / High / Medium / Low
- LOCATION: file:line
- ISSUE: description
- SUGGESTION: how to fix

Use dependency list to check for broader impact.
"""
disallowed_tools = ["Write", "Edit"]

[mode.fix]
aliases = ["f"]
output_states = ["fixed", "unfixable"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Fix `{target}`: {description}

{ide_context}

1. Read the code and understand the issue
2. Check dependencies for impact of fix
3. Implement minimal, targeted fix
4. Run related tests if available
5. Set state to "fixed" or "unfixable"
"""
system_prompt = """
You fix bugs. Minimal, surgical changes only.

DO:
- Fix the root cause
- Keep changes small
- Match existing code style
- Verify fix with related tests

DON'T:
- Refactor surrounding code
- Add features while fixing
- Change public APIs unless necessary
"""

[mode.implement]
aliases = ["i", "impl"]
output_states = ["implemented", "blocked"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Implement at `{target}`: {description}

{ide_context}

1. Read surrounding code and dependencies to understand patterns
2. Implement the functionality
3. Handle errors appropriately
4. Set state to "implemented" or "blocked"
"""
system_prompt = """
You implement features. Match existing codebase style and patterns.

DO:
- Reuse existing utilities from dependencies
- Handle errors consistently
- Write clear, idiomatic code

DON'T:
- Over-engineer
- Add unnecessary dependencies
- Break existing functionality
"""

[mode.optimize]
aliases = ["o", "opt"]
output_states = ["optimized"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Optimize `{target}`: {description}

{ide_context}

1. Read the code and analyze call patterns from dependencies
2. Identify actual bottlenecks
3. Apply targeted optimizations
4. Run related tests to verify correctness
5. Set state to "optimized"
"""
system_prompt = """
You optimize code for performance. Never sacrifice correctness.

FOCUS ON:
- Algorithm complexity (O(n²) → O(n log n))
- Data structure choice
- Reducing allocations
- Caching and batching

DO:
- Use dependency info to understand hot paths
- Document tradeoffs
- Preserve exact behavior

DON'T:
- Premature micro-optimizations
- Sacrifice readability for minor gains
"""

[mode.explain]
aliases = ["e", "exp"]
output_states = ["explained"]
prompt = """
Explain `{target}`: {description}

{ide_context}

1. Read and understand the code
2. Explain what it does and how it connects to dependencies
3. Set state to "explained"
"""
system_prompt = """
You explain code. READ-ONLY - no edits.

STRUCTURE:
- One-sentence summary first
- Step-by-step breakdown of logic
- How it connects to listed dependencies
- Key patterns and concepts used
- Non-obvious behavior or gotchas

Explain the "why", not just the "what".
"""
disallowed_tools = ["Write", "Edit"]

[mode.commit]
aliases = ["cm", "git"]
output_states = ["committed"]
allowed_tools = ["Bash(git status:*)", "Bash(git diff:*)", "Bash(git add:*)", "Bash(git commit:*)", "Bash(git log:*)", "Read"]
disallowed_tools = ["Write", "Edit"]
prompt = """
Commit staged changes: {description}

1. Run `git diff --cached` to review changes
2. Determine commit type and write message
3. Execute commit and set state to "committed"
"""
system_prompt = """
You create git commits. Use conventional commits format.

FORMAT: <type>(<scope>): <subject>

TYPES: feat, fix, docs, style, refactor, perf, test, build, ci, chore

RULES:
- Max 72 chars subject, imperative mood ("Add" not "Added")
- Warn if sensitive files staged (.env, credentials)
- Never amend or force push without explicit request
"""

[mode.decouple]
aliases = ["dec", "inject", "di"]
output_states = ["decoupled"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Decouple dependency at `{target}`: {description}

{ide_context}

1. Identify the direct dependency to abstract
2. Create an interface/trait for the dependency
3. Inject the dependency instead of hardcoding
4. Update all usages in listed dependencies
5. Set state to "decoupled"
"""
system_prompt = """
You decouple code by introducing abstractions. Enable testability and flexibility.

DO:
- Create interface/trait matching current usage
- Use constructor/parameter injection
- Update all callers from dependency list
- Keep interface minimal

DON'T:
- Over-abstract (one interface per concrete type is usually wrong)
- Change behavior while decoupling
- Add unused interface methods
"""

[mode.extract]
aliases = ["ex", "split"]
output_states = ["extracted"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Extract from `{target}`: {description}

{ide_context}

1. Identify the code to extract
2. Create new function/module/service
3. Replace original with call to extracted code
4. Update imports in dependencies
5. Set state to "extracted"
"""
system_prompt = """
You extract code into reusable units. Improve modularity.

DO:
- Give clear, descriptive names
- Define clean interface (minimal parameters)
- Place in appropriate location (same file, new file, new module)
- Update all callers from dependency list

DON'T:
- Extract code only used once (unless for clarity)
- Create deep call hierarchies
- Change behavior while extracting
"""

[mode.logging]
aliases = ["log", "l"]
output_states = ["logged"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Add logging to `{target}`: {description}

{ide_context}

1. Identify the existing logging framework
2. Add structured logging at appropriate points
3. Use correct log levels (error, warn, info, debug, trace)
4. Set state to "logged"
"""
system_prompt = """
You add logging. Use the project's existing logging framework.

LOG LEVELS:
- error: failures requiring attention
- warn: unexpected but handled situations
- info: significant business events
- debug: diagnostic information
- trace: detailed execution flow

DO:
- Include relevant context (IDs, parameters)
- Use structured logging (key=value)
- Log at entry/exit of important operations

DON'T:
- Log sensitive data (passwords, tokens, PII)
- Over-log in hot paths (performance impact)
- Use string concatenation for log messages
"""

[mode.security]
aliases = ["sec", "harden"]
output_states = ["secured", "vulnerabilities_remain"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Secure `{target}`: {description}

{ide_context}

1. Analyze code for security vulnerabilities
2. Fix identified issues
3. Add input validation where missing
4. Set state to "secured" or "vulnerabilities_remain"
"""
system_prompt = """
You fix security issues. OWASP Top 10 focus.

CHECK AND FIX:
- Injection (SQL, XSS, command, path traversal)
- Auth issues (broken auth, missing checks)
- Data exposure (logging secrets, insecure storage)
- Insecure defaults (weak crypto, permissive CORS)

DO:
- Validate and sanitize all inputs
- Use parameterized queries
- Encode outputs appropriately
- Apply principle of least privilege

DON'T:
- Security through obscurity
- Roll your own crypto
- Trust client-side validation alone
"""

[mode.types]
aliases = ["ty", "typing"]
output_states = ["typed"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep"]
prompt = """
Add types to `{target}`: {description}

{ide_context}

1. Analyze the code and infer types
2. Add type annotations matching project style
3. Fix any type errors introduced
4. Set state to "typed"
"""
system_prompt = """
You add type annotations. Improve type safety.

DO:
- Use specific types (not any/unknown unless necessary)
- Add return types to functions
- Type function parameters
- Create interfaces/types for complex objects
- Match existing project type patterns

DON'T:
- Over-type obvious literals
- Use overly complex generic types
- Add types that reduce flexibility without benefit
"""

[mode.coverage]
aliases = ["cov"]
output_states = ["coverage_improved"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Improve test coverage for `{target}`: {description}

{ide_context}

1. Identify untested code paths
2. Write tests for uncovered branches
3. Run tests and verify coverage improved
4. Set state to "coverage_improved"
"""
system_prompt = """
You write tests for uncovered code. Target specific gaps.

PRIORITIZE:
- Error handling paths
- Edge cases and boundary conditions
- Complex conditional branches
- Integration points

DO:
- Check related tests for patterns
- Focus on behavior, not implementation
- Test one thing per test

DON'T:
- Write tests just for coverage numbers
- Test trivial getters/setters
- Duplicate existing test coverage
"""

[mode.migrate]
aliases = ["mig", "upgrade"]
output_states = ["migrated", "migration_blocked"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Migrate `{target}`: {description}

{ide_context}

1. Understand the migration requirements
2. Update code to new API/version
3. Update all usages in dependencies
4. Run tests to verify migration
5. Set state to "migrated" or "migration_blocked"
"""
system_prompt = """
You migrate code to new APIs/versions. Ensure compatibility.

DO:
- Read migration guides for the target version
- Update all affected files from dependency list
- Handle deprecated features appropriately
- Test thoroughly after migration

DON'T:
- Mix old and new patterns inconsistently
- Ignore deprecation warnings
- Migrate without understanding breaking changes
"""

[mode.cleanup]
aliases = ["clean", "tidy"]
output_states = ["cleaned"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
prompt = """
Clean up `{target}`: {description}

{ide_context}

1. Identify dead code, unused imports, obsolete comments
2. Remove or fix identified issues
3. Verify nothing breaks via dependencies and tests
4. Set state to "cleaned"
"""
system_prompt = """
You clean up code. Remove cruft, keep functionality.

REMOVE:
- Unused imports and variables
- Dead code (unreachable, commented out)
- Obsolete TODOs and FIXMEs
- Redundant type casts

DO:
- Verify removal won't break dependents
- Run tests after cleanup
- Keep meaningful comments

DON'T:
- Remove code that looks unused but isn't (reflection, dynamic)
- Delete TODOs without checking if still relevant
- Clean up code you don't understand
"""

# ============================================================================
# CHAINS - Sequential mode execution with state-based triggers
# ============================================================================
#
# Chains execute multiple modes in sequence. Each step can:
# - Run unconditionally
# - Trigger only when the previous step's state matches `trigger_on`
# - Skip when the previous step's state matches `skip_on`
#
# The output summary from each step is passed as context to the next step,
# giving each agent fresh context and the accumulated knowledge from prior steps.
#
# Example workflow: review-and-fix
#   1. Review code → outputs state "issues_found" or "no_issues"
#   2. Fix issues (only if "issues_found") → outputs state "fixed"
#   3. Run tests (only if "fixed") → outputs state "tests_pass" or "tests_fail"

[chain.review-and-fix]
description = "Review code, fix any issues found, then run tests"
stop_on_failure = true
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
    { mode = "tests", trigger_on = ["fixed"] },
]

[chain.implement-and-test]
description = "Implement a feature, then write and run tests"
stop_on_failure = true
steps = [
    { mode = "implement" },
    { mode = "tests", trigger_on = ["implemented"] },
]

[chain.full-review]
description = "Review, fix, test, then document"
stop_on_failure = false
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
    { mode = "tests", trigger_on = ["fixed"] },
    { mode = "docs", skip_on = ["tests_fail"] },
]

[chain.refactor-safe]
description = "Review first, then refactor, then test"
stop_on_failure = true
steps = [
    { mode = "review" },
    { mode = "refactor", trigger_on = ["no_issues"] },
    { mode = "tests", trigger_on = ["refactored"] },
]

[chain.secure-and-test]
description = "Security audit, fix vulnerabilities, then verify with tests"
stop_on_failure = true
steps = [
    { mode = "security" },
    { mode = "tests", trigger_on = ["secured"] },
]

[chain.decouple-and-test]
description = "Decouple dependencies, then verify with tests"
stop_on_failure = true
steps = [
    { mode = "decouple" },
    { mode = "tests", trigger_on = ["decoupled"] },
]

[chain.extract-and-test]
description = "Extract code into module/service, then test"
stop_on_failure = true
steps = [
    { mode = "extract" },
    { mode = "tests", trigger_on = ["extracted"] },
]

[chain.modernize]
description = "Add types, cleanup dead code, then test"
stop_on_failure = false
steps = [
    { mode = "types" },
    { mode = "cleanup", trigger_on = ["typed"] },
    { mode = "tests", trigger_on = ["cleaned"] },
]

[chain.harden]
description = "Security fix, add logging, then test"
stop_on_failure = true
steps = [
    { mode = "security" },
    { mode = "logging", trigger_on = ["secured"] },
    { mode = "tests", trigger_on = ["logged"] },
]

[chain.quality-gate]
description = "Full quality check: review, security, types, coverage"
stop_on_failure = false
steps = [
    { mode = "review" },
    { mode = "security", trigger_on = ["issues_found"] },
    { mode = "types" },
    { mode = "coverage" },
]

[chain.refactor-full]
description = "Extract, decouple, refactor, then test"
stop_on_failure = true
steps = [
    { mode = "extract" },
    { mode = "decouple", trigger_on = ["extracted"] },
    { mode = "refactor", trigger_on = ["decoupled"] },
    { mode = "tests", trigger_on = ["refactored"] },
]

[chain.implement-full]
description = "Implement, add types, logging, docs, then test"
stop_on_failure = true
steps = [
    { mode = "implement" },
    { mode = "types", trigger_on = ["implemented"] },
    { mode = "logging", trigger_on = ["typed"] },
    { mode = "docs", trigger_on = ["logged"] },
    { mode = "tests", trigger_on = ["documented"] },
]

[chain.migrate-safe]
description = "Review, migrate, test, then cleanup"
stop_on_failure = true
steps = [
    { mode = "review" },
    { mode = "migrate", trigger_on = ["no_issues"] },
    { mode = "tests", trigger_on = ["migrated"] },
    { mode = "cleanup", trigger_on = ["tests_pass"] },
]

[chain.cleanup-full]
description = "Cleanup, review for issues, fix if needed"
stop_on_failure = false
steps = [
    { mode = "cleanup" },
    { mode = "review", trigger_on = ["cleaned"] },
    { mode = "fix", trigger_on = ["issues_found"] },
    { mode = "tests", trigger_on = ["fixed"] },
]
"#;

/// Initialize a new KYCo configuration
pub async fn init_command(work_dir: &Path, force: bool) -> Result<()> {
    let config_dir = work_dir.join(".kyco");
    let config_path = config_dir.join("config.toml");

    // Check for existing config (both new and legacy locations)
    let legacy_path = work_dir.join("kyco.toml");

    if config_path.exists() && !force {
        bail!(
            "Configuration already exists: {}\nUse --force to overwrite.",
            config_path.display()
        );
    }

    if legacy_path.exists() && !force {
        bail!(
            "Legacy configuration exists: {}\nMove it to {} or use --force to create new config.",
            legacy_path.display(),
            config_path.display()
        );
    }

    // Create .kyco directory
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    // Write config
    std::fs::write(&config_path, DEFAULT_CONFIG)?;
    println!("Created: {}", config_path.display());

    Ok(())
}
