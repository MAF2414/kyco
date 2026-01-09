# VULN-006: Codex Bridge läuft ohne Tool-Approvals (approval_policy = Default "Never")

## Summary
`CodexBridgeAdapter` setzt `approval_policy` in `CodexQueryRequest` nie, wodurch die Bridge/Codex im Default `Never` (Auto-Approve) läuft. Dadurch werden Tool-Calls (inkl. Shell/Write) ohne User-Gate ausgeführt und Prompt-Injection/Job-Inputs können unmittelbar zu unerwarteten Änderungen/Command-Execution führen.

## Severity
**HIGH** - Auto-approvte Tool-Ausführung unterminiert das Permission-Modell; ein Angreifer kann über Prompt-Injection oder untrusted Job-Inputs ohne explizite Freigabe Dateien ändern/Commands ausführen (mindestens im Workspace-Sandbox).

## Location
src/agent/bridge/adapter/codex.rs:88-103
src/agent/bridge/adapter/codex.rs:119-128
src/agent/bridge/types/mod.rs:159-171
src/agent/bridge/types/mod.rs:202-207

## Code
```rust
// src/agent/bridge/adapter/codex.rs
CodexQueryRequest {
    // ...
    approval_policy: None,
    // ...
}
```

```rust
// src/agent/bridge/types/mod.rs
pub enum CodexApprovalPolicy {
    /// Auto-approve all tool use (default for backward compatibility)
    Never,
    // ...
}
// ...
/// Approval policy for tool use (default: Never for backward compatibility)
pub approval_policy: Option<CodexApprovalPolicy>,
```

## Impact
- Unerwartete Tool-Ausführung ohne Nachfrage: Shell/Write/Edit laufen ohne explizite Bestätigung.
- Prompt-Injection wird deutlich gefährlicher: Angreifer kann Codex dazu bringen, Arbeitsbereich zu manipulieren, Secrets aus Repo-Dateien zu exfiltrieren oder CI/Hook-Payloads zu droppen (abhängig von Sandbox/Tooling).

## Attack Scenario
1. Angreifer platziert Prompt-Injection in einer Datei, die der User typischerweise „review/refactor“ lässt (z.B. `README.md`, PR-Description, Kommentar-Block).
2. User startet einen Job mit `agent=codex` (Bridge) gegen das Repo.
3. Codex folgt der Injection und ruft Tools wie `Bash`/`Write` auf (z.B. `cat .env` → exfil via HTTP, oder Code-Backdoor schreiben).
4. Da `approval_policy` effektiv `Never` ist, werden Tool-Calls ohne UI-Approval ausgeführt.

## Suggested Fix
- `approval_policy` konfigurierbar machen (z.B. Feld in `AgentConfig`) und in `CodexQueryRequest` setzen.
- Default auf ein sichereres Profil ändern (z.B. `OnFailure` oder `UnlessAllowListed`/`Always`) und im GUI/Bridge eine Approval-Flow für Codex implementieren, statt implizit `Never`.
- Optional: Wenn keine Approval-Implementierung vorhanden ist, Codex-Bridge nur mit `read-only`/restriktiver Sandbox als Default starten oder den `codex` Bridge-Agent in configs explizit als „unattended“ markieren.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben

