# KYCo BugBounty – TODO Liste (aus `plan.md`)

_Letztes Update: 2026-01-17_

Diese Liste ist eine **Checkliste** der Anforderungen aus `plan.md` (root) und spiegelt den aktuellen Stand im Repo wider (siehe auch `KYCo/work.md`).

## Done (bereits umgesetzt)

### Data Layer / Persistenz
- [x] SQLite BugBounty DB + Migrationen (`~/.kyco/bugbounty.db`)
- [x] Models + Repositories: Projects, Findings, Artifacts, FlowEdges, Jobs, Job↔Finding Links
- [x] Finding IDs: `${project_id}-VULN-###`

### CLI – Projects/Findings
- [x] `kyco project discover/list/show/select/init/delete/overview`
- [x] `kyco finding list/show/create/set-status/fp/delete/export`
- [x] `kyco finding import` (SARIF/Semgrep/Snyk/Auto inkl. Nuclei)
- [x] `kyco finding import-notes` (sync aus `notes/findings/*.md` → DB)
- [x] `kyco finding extract-from-job`

### Jobs – BugBounty Integration
- [x] Project inference aus File-Path (best effort)
- [x] Prompt-Injection: Scope/ToolPolicy/Known Findings + Output Contract in Job-Context
- [x] Persistierte BugBounty Jobs + Job completion metadata (`result_state`, `next_context_json`)
- [x] ToolPolicy → harte Tool-Blocks via `disallowed_tools`
- [x] Job Output Parsing: `next_context` ingestion → Findings/FlowEdges/Artifacts + Job↔Finding “discovered” Links

### CLI – Job Inspection (GUI /ctl)
- [x] `kyco job list --project <id>` (Filter via `job.bugbounty_project_id`)
- [x] `kyco job output --findings/--flow` (parsed `next_context.*`)

### GUI – Kanban
- [x] BugBounty Kanban View (Projects → Columns → Drag&Drop Status)
- [x] Finding Detail: Flow Graph rendering
- [x] Linked Jobs: Job-Count auf Cards + Linked-Job-Liste im Detailpanel

---

## TODO (alles erledigt)

### A) Entscheidungen / Grundsatzfragen (blocker)
- [x] **Global DB**: BugBounty DB ist global `~/.kyco/bugbounty.db` (keine per-workspace DB files)
- [x] **Filesystem SSoT**: definiert in `KYCo/docs/bugbounty_notes.md`

### B) Persistenz / Job Backlog (P0)
- [x] Persistenter Backlog für Jobs (pending/queued/running/done) über GUI-Sessions hinweg (JobManager Snapshot in `.kyco/job_manager.json`)
- [x] Konsistente IDs: Mapping KYCo JobId ↔ BugBountyJobId für History/Links auch ohne laufende GUI (IDs werden aus Snapshot weitergeführt)

### C) CLI – Projects (P1)
- [x] `kyco project list` mit Counters (jobs open, findings open, last activity)
- [x] `kyco project show` mit SQL-Aggregationen + “last activity” (ohne full-scan)
- [x] `kyco project discover` erweitert um Custom-Metadaten (Stack/Auth/Links)

### D) CLI – Findings (P0/P1)
- [x] Finding↔Job manuell linken/unlinken: `kyco finding link --finding <fid> --job <job_id>` + `unlink`
- [x] `kyco finding list` Filter kombinierbar machen (`--project` + `--status` + `--severity` + `--search`)
- [x] `kyco finding create --write-notes` schreibt ein Template nach `notes/findings/<fid>.md`
- [x] Notes Sync bidirektional:
  - [x] `kyco finding export-notes <fid>` (DB → `notes/findings/<fid>.md`)
  - [x] Konfliktstrategie (DB vs File geändert) + “dry-run diff”

### E) CLI – Jobs (P0/P1)
- [x] `kyco job start --project <id>` (setzt Projekt für Prompt-only Jobs; Scope-Lock noch offen)
- [x] `kyco job start --finding <fid>...` (repeatable) → verlinkt Job + inject finding context
- [x] `kyco job list --finding <fid>` (über `job_findings`)
- [x] `kyco job list --state <result.state>` (nicht nur JobStatus)
- [x] Job-Output Convenience:
  - [x] `kyco job output --next-context` (kompletter parsed Block)
  - [x] `kyco job output --artifacts`

### F) GUI – Project Dashboard & Jobs View (P1)
- [x] Project Switcher + Dashboard (Jobs: pending/queued/running/done/failed; Findings: severity/status; Recent activity)
- [x] Dedicated “Jobs View” mit Filter/Sort (project, finding, state, agent, file)
- [x] Finding Detail: Quick Actions (“Start Verify Job”, “Start Flow Trace”, “Export Report”, “Mark FP”) inkl. Job↔Finding Links

### G) Scope & Safety Hardening (P0)
- [x] CLI Scope Commands:
  - [x] `kyco scope show [--project <id>]`
  - [x] `kyco scope check <url>` inkl. “warum/warum nicht” (In-/Out-of-scope reasoning)
  - [x] `kyco scope policy [--project <id>]`
- [x] Harte Scope-Enforcement im Tooling (nicht nur Prompt):
  - [x] Domain allowlist/denylist Enforcement bei Netzwerk-Tools/Wrappern
  - [x] Rate-Limit Enforcement (per domain)
  - [x] “Wrapper only” Enforcement für `curl/wget/...` (Policy-Wrapper Pflicht)
- [x] Secret Protection:
  - [x] `protected_paths` / `block_agent_read` durchgesetzt (Tool-layer)
  - [x] Output redaction (Tokens/Cookies) als Guardrail

### H) Agent Output Contract / Profiles (P0/P1)
- [x] `.kyco/profiles/security-audit.yaml` + **strict enforcement** für BugBounty Security-Skills (missing required fields ⇒ Job failed)
- [x] Output Schema Erweiterungen:
  - [x] FlowEdges: per-edge `finding_id` (Fallback: erstes Finding)
  - [x] Artifacts: optional `finding_id` + `hash` inkl. best-effort Dedupe
- [x] Chains:
  - [x] `audit-file` / `audit-project` orchestration + aggregator job (BugBounty-aware)

### I) Tool Integrations / Imports (P1/P2)
- [x] Snyk Import
- [x] `kyco import <tool>` Alias-Commands (semgrep/codeql/snyk/nuclei/sarif/auto)
- [x] “Import → Job Generator”: pro imported finding automatisch einen Verify-Job anlegen + job↔finding verlinken (via GUI `/ctl`)
- [x] Recon Artefakte als Inputs (`kyco job start --input ... --batch`) + Batch-Erstellung

---

## Entscheidungen (aus dem Plan)
- [x] Projektgrenzen: 1 BugBounty Program = 1 Project root (MVP); Sub-Projects später als eigene Projects
- [x] Sync-Mechanik: on-demand via `kyco finding import-notes` / `kyco finding export-notes` (Conflict-Schutz via mtime + `--force`)
- [x] Secrets Default: `project discover` inferiert `protected_paths` (z.B. `auth/`) + Bridge-Redaction ist bei BugBounty Jobs aktiv
- [x] Kanban Customization: fixe Columns über `FindingStatus` (MVP); user-defined workflow später
- [x] Multi-User / Team-Sharing: DB lokal/global; Team-Sharing primär über git/Notes, DB-Sync später
