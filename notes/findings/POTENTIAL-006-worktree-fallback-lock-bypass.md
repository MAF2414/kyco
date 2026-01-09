# POTENTIAL-006: Worktree-Fallback umgeht File-Locking (Race/Repo-Korruption)

## Summary
Wenn `settings.use_worktree` aktiviert ist, überspringt der Executor das per-File Locking (in Erwartung von Worktree-Isolation). `setup_worktree()` kann jedoch in mehreren Fällen stillschweigend auf “in-place” Ausführung ohne Worktree zurückfallen, wodurch Jobs parallel ohne Locks im gleichen Workspace laufen können.

## Severity
**MEDIUM** - Bricht die intendierte Isolation und erlaubt Race-Conditions zwischen parallelen Jobs (inkonsistente/korruptierte Working-Tree-Änderungen). Bei angreifbarer lokalen Control-API (z.B. Auth deaktiviert) kann ein Angreifer gezielt konkurrierende Jobs auf dasselbe File queue’n.

## Location
src/gui/executor/mod.rs:136-170
src/gui/executor/run_job.rs:62-115
src/gui/executor/worktree_setup.rs:24-98

## Code
```rust
// src/gui/executor/mod.rs
let needs_lock_check =
    !should_use_worktree && !is_multi_agent && !job.force_worktree;

if needs_lock_check {
    // ...
    manager.try_lock_file(&job.source_file, job.id);
}
```

```rust
// src/gui/executor/run_job.rs
let should_use_worktree = match mode_use_worktree {
    Some(true) => true,
    Some(false) => false,
    None => config.settings.use_worktree || is_multi_agent_job || job.force_worktree,
};

let (worktree_path, is_in_worktree) =
    if let Some(existing_worktree) = job.git_worktree_path.take().filter(|p| p.exists()) {
        (existing_worktree, true)
    } else if should_use_worktree {
        match setup_worktree(/* ... */) {
            Some(result) => result,
            None => return,
        }
    } else {
        (job_work_dir, false)
    };
```

```rust
// src/gui/executor/worktree_setup.rs
if let Some(git) = git_manager {
    match git.create_worktree(job_id) {
        Err(e) => {
            if is_multi_agent_job || force_worktree {
                return None;
            }
            // Fallback to in-place execution (no worktree)
            Some((job_work_dir.clone(), false))
        }
        // ...
    }
} else if is_multi_agent_job || force_worktree {
    None
} else {
    // No git repo available -> in-place execution (no worktree)
    Some((job_work_dir.clone(), false))
}
```

## Impact
- Parallele Jobs können im selben Arbeitsverzeichnis ohne Mutual Exclusion laufen, obwohl “Worktree-Isolation” konfiguriert ist.
- Das “one job per file”-Sicherheitsnetz (File Locks) greift in diesen Fallback-Fällen nicht.
- Ergebnis: Race-Conditions, inkonsistente Diffs/Edits, Repo/Working-Tree-Korruption; im Worst-Case kann ein Angreifer gezielt “conflicting writes” erzeugen und unerwartete/unerwünschte Code-Änderungen im Workspace hinterlassen.

## Attack Scenario
1. Victim setzt `settings.use_worktree = true` (z.B. für Isolation), arbeitet aber in einem nicht-Git-Workspace oder in einem Repo, in dem `git.create_worktree()` sporadisch fehlschlägt (z.B. keine Commits / Permissions / sonstige Git-Fehler).
2. Angreifer (lokal oder via localhost-CSRF wenn `/ctl/*` Auth deaktiviert ist) triggert mehrere Jobs parallel via `POST /ctl/jobs`, die dasselbe `source_file` adressieren.
3. `executor_loop` überspringt das File-Locking, weil `should_use_worktree == true`.
4. `setup_worktree()` fällt auf `(job_work_dir, false)` zurück → alle Jobs laufen “in-place” im selben Workspace ohne Locks und schreiben konkurrierend ins gleiche File/Repo.

## Suggested Fix
- Locking-Entscheidung an das *tatsächliche* Ausführungsmodell koppeln (ob `is_in_worktree` am Ende true/false ist), nicht nur an `settings.use_worktree`.
- Wenn `settings.use_worktree == true`, aber kein Worktree erzeugt werden kann: entweder
  - Job fail-closed abbrechen, oder
  - explizit auf “in-place + File-Lock” downgraden (Lock vor Start setzen, sonst Job blocken/queue’n).
- “Will_use_worktree”/Isolation-Entscheidung zentralisieren, damit `executor_loop` und `run_job` nicht divergieren können.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben

