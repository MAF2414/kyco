# KYCo BugBounty Implementation - Work Coordination

Dieses File dient zur Abstimmung zwischen Claude und Codex Agents.

---

## Status: Phases 1-4 ✅ / Integration 🚧

| Agent | Arbeitet an | Status | Letzte Aktivität |
|-------|-------------|--------|------------------|
| Claude | Phase 1-4 COMPLETE | ✅ Done | 2025-01-17 11:45 |
| Codex | Integration polish (CLI/GUI job context) | ✅ Done | 2026-01-17 12:15 |

---

## Current Claims (to avoid conflicts)

| Agent | Claim | File/Dir scope | Status |
|-------|-------|----------------|--------|
| Codex | BugBounty ↔ Job execution integration (prompt injection + next_context ingestion + active project persistence) | `KYCo/src/gui/executor/run_job.rs`, `KYCo/src/bugbounty/**`, `KYCo/src/cli/project.rs` | ✅ Done (2026-01-17) |
| Codex | BugBounty jobs persistence + tool-policy enforcement + notes import | `KYCo/src/bugbounty/**`, `KYCo/src/gui/executor/run_job.rs`, `KYCo/src/cli/finding.rs`, `KYCo/src/commands.rs`, `KYCo/src/main.rs` | ✅ Done (2026-01-17) |
| Codex | CLI/GUI visibility for BugBounty job history (overview, linked jobs, job output next_context) | `KYCo/src/cli/project.rs`, `KYCo/src/cli/finding.rs`, `KYCo/src/cli/job/**`, `KYCo/src/gui/kanban/**`, `KYCo/src/commands.rs`, `KYCo/src/main.rs` | ✅ Done (2026-01-17) |
| Claude | (fill in) | (fill in) | (fill in) |

---

## Completed Work

### Phase 1: Data Layer ✅ (Claude)

- [x] `src/bugbounty/db.rs` - SQLite Schema + Migrations
- [x] `src/bugbounty/models/*.rs` - Project, Finding, Artifact, FlowEdge Models
- [x] `src/bugbounty/repository.rs` - CRUD Operations
- [x] `src/bugbounty/mod.rs` - BugBountyManager public API
- [x] Finding IDs sind global-unique: `${project_id}-VULN-###`

### Phase 1: CLI Commands ✅ (Claude)

- [x] `src/cli/finding.rs` - Finding CLI (list/show/create/set-status/fp/delete/export)
- [x] `src/cli/project.rs` - Project CLI (list/show/discover/select/init/delete/overview)
- [x] `src/commands.rs` - FindingCommands + ProjectCommands enums

### Phase 2: Finding-first Workflow ✅ (Claude)

- [x] `src/bugbounty/next_context.rs` - Parser für `next_context.findings[]`
- [x] `src/bugbounty/context_injector.rs` - Auto-inject known findings in jobs

### Phase 2: Security Skills ✅ (Claude)

- [x] `.claude/skills/authz-bypass-hunter/SKILL.md`
- [x] `.claude/skills/injection-hunter/SKILL.md`
- [x] `.claude/skills/secrets-hunter/SKILL.md`
- [x] `.claude/skills/crypto-audit/SKILL.md`
- [x] `.claude/skills/jwt-attack-surface/SKILL.md`
- [x] `.claude/skills/dos-resource-exhaustion/SKILL.md`
- [x] `.claude/skills/go-security-audit/SKILL.md`
- [x] `.claude/skills/flow-trace/SKILL.md`

### Phase 3: Scope & Policy ✅ (Codex)

- [x] `src/bugbounty/scope_parser.rs` - scope.md Parser
- [x] `src/cli/project.rs` - Scope + ToolPolicy integration in discover

### Phase 4: Overview & Reporting ✅ (Claude)

- [x] `kyco project overview` command
- [x] Markdown + JSON output formats
- [x] BugBounty/OVERVIEW.md auto-update

---

## Available CLI Commands

```bash
# Project Management
kyco project list [--platform <plat>] [--json]
kyco project show <id> [--json]
kyco project discover [--path <dir>] [--dry-run]  # Parses scope.md automatically
kyco project init --id <id> --path <path> [--platform <plat>]
kyco project select <id>
kyco project delete <id> [-y]
kyco project overview [-p <project>] [-o <file>] [--update-global] [--json]

# Finding Management (Kanban)
kyco finding list [-p <project>] [-s <status>] [--severity <sev>] [--search <q>] [--json]
kyco finding show <id> [--json]
kyco finding create -t <title> -p <project> [options...] [--write-notes]
kyco finding set-status <id> <status>
kyco finding link --finding <fid> --job <job_id|#job_id> [--link-type related]
kyco finding unlink --finding <fid> --job <job_id|#job_id>
kyco finding fp <id> -r <reason>
kyco finding delete <id> [-y]
kyco finding export <id> -f markdown|intigriti|hackerone [-o <file>]
kyco finding export-notes <id> [--dry-run] [--force] [--json]

# Job Inspection (talks to running GUI via /ctl)
kyco job list [--project <id>] [--finding <fid>] [--status <status>] [--state <state>] [--skill <skill>] [--json]
kyco job output <job_id> [--next-context] [--findings] [--flow] [--artifacts] [--summary] [--state] [--json]
```

## Available Security Skills

```bash
# Use with: kyco job start --skill <name> --file <file> [--project <id>] [--finding <fid>,...]
authz-bypass-hunter    # IDOR, privilege escalation, access control
injection-hunter       # SQLi, XSS, command injection, SSTI
secrets-hunter         # Hardcoded keys, tokens, credentials
crypto-audit           # Weak crypto, key management, timing attacks
jwt-attack-surface     # JWT algorithm confusion, validation issues
dos-resource-exhaustion # ReDoS, memory exhaustion, unbounded ops
go-security-audit      # Go-specific security patterns
flow-trace             # Cross-file taint analysis
```

---

## Integration Points

### API Usage

```rust
use kyco::bugbounty::{
    BugBountyManager, Project, Finding, FindingStatus, Severity,
    NextContext, ContextInjector, parse_scope_file
};

let manager = BugBountyManager::new()?;

// Process agent output
let ctx = NextContext::extract_from_text(agent_output)?;
let finding_ids = manager.process_next_context("project-id", &ctx, Some("job-id"))?;

// Inject context into prompts
let injector = ContextInjector::new(manager.clone());
let context = injector.for_project("project-id")?;
let system_prompt_addon = context.to_system_prompt();
```

### Database Location

```
~/.kyco/bugbounty.db  (separate from existing stats.db)
```

---

## Open Items for Future

- [x] Persist BugBounty job backlog (queued/pending) across GUI sessions
- [x] Notes sync in both directions (DB ↔ `notes/findings/*.md`)
- [x] Tool-layer scope enforcement + wrapper-only + rate-limits + redaction
- [ ] Optional: restrict file access to project root (allowlist), not only protected paths
- [ ] Optional: watcher-based sync for notes (on-demand bleibt Default)
- [ ] Optional: kanban workflow customization (user-defined columns)
