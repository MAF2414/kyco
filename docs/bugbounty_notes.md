# BugBounty Notes (Filesystem SSoT)

KYCo kann Findings sowohl in der globalen BugBounty-DB (`~/.kyco/bugbounty.db`) als auch als Markdown-Dateien im Filesystem verwalten (`notes/findings/*.md` unter dem jeweiligen Project Root).

Dieses Dokument definiert, **welche Felder im Filesystem die Source of Truth (SSoT)** sind und welche Daten **DB-only** bleiben.

## SSoT: `notes/findings/<FINDING_ID>.md`

Die Notes-Datei ist die Source of Truth für den **Finding-Inhalt**:

- `title` (Heading `# <id>: <title>`)
- `severity` (`**Severity:** ...`)
- `status` (`**Status:** ...`)
- `confidence` (`**Confidence:** ...`)
- `reachability` (`**Reachability:** ...`)
- `cwe_id` (`**CWE:** ...`)
- `cvss_score` (`**CVSS:** ...`)
- `fp_reason` (`**FP Reason:** ...`)
- `attack_scenario` (`## Attack Scenario`)
- `preconditions` (`## Preconditions`)
- `impact` (`## Impact`)
- `affected_assets[]` (`## Affected Assets`)
- `taint_path` (`## Flow`)
- `notes` (`## Notes`)

### “Clearing” von Feldern

- Inline-Felder: setze `-` um einen Wert zu löschen (z.B. `**CWE:** -`).
- Text-Sektionen: lasse die Sektion leer, um den DB-Wert zu löschen.
- Listen-Sektionen: lasse die Liste leer, um die Liste zu leeren.

## DB-only (nicht im Filesystem)

Diese Daten bleiben **DB-only** und werden nicht aus Notes-Dateien übernommen:

- Timestamps: `created_at`, `updated_at` (DB/Sync-Metadaten)
- Job-Historie (`jobs`), KYCo Job-Mapping (`kyco_job_id`)
- Job↔Finding Links (`job_findings`)
- Artifacts / Evidence (`artifacts`)
- FlowEdges / FlowTrace (`flow_edges`)
- Projekt-Daten (Scope/ToolPolicy/Metadata) – kommen aus Project-Setup/`scope.md`

## Sync Semantik (CLI)

### File → DB: `kyco finding import-notes --project <id>`

- Updated DB-Felder nur wenn das Feld im Notes-File **explizit vorhanden** ist.
- Leere Sektion / `-` wird als “clear” interpretiert.
- Fehlende Sektion/Key (legacy Notes) wird als “nicht anfassen” behandelt.

### DB → File: `kyco finding export-notes <finding_id>`

- Schreibt/überschreibt `notes/findings/<finding_id>.md` (Template aus DB).
- Konflikt-Schutz: wenn die Datei “neuer” wirkt als die DB (`mtime > finding.updated_at`) wird abgebrochen, außer `--force`.
- Auflösen: zuerst `kyco finding import-notes --project <id>` ausführen oder explizit `--force` nutzen.

