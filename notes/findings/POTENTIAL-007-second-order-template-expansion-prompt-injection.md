# POTENTIAL-007: Second-order Template Expansion enables prompt manipulation

## Summary
`Config::build_prompt()` führt mehrere `.replace()`-Passes nacheinander aus. Dadurch werden Placeholder-Tokens, die in bereits eingesetzten (potenziell untrusted) Werten wie `target_text`/`scope_text`/`file` enthalten sind, in späteren Replacements erneut interpretiert (“second-order template expansion”).

## Severity
**MEDIUM** - Wenn ein Angreifer `target`/`scope`/`file` (z.B. über untrusted Repos, ungewöhnliche Dateinamen oder Control-API Inputs) beeinflussen kann, kann er Prompt-Struktur/Quoting brechen und zusätzliche Anweisungen an höher priorisierten Stellen platzieren; in Kombination mit Auto-Run/Tooling kann das zu unerwarteten Writes/Command-Ausführung führen.

## Location
src/config/lookup.rs:201-237
src/agent/chain/prompt.rs:42

## Code
```rust
// src/config/lookup.rs
template
    .replace("{mode}", mode)
    .replace("{target}", target_text)
    .replace("{scope}", scope_text)
    .replace("{file}", file)
    .replace("{description}", description)
```

## Impact
- Ein untrusted `target_text`/`scope_text`/`file` kann Tokens wie `{description}`/`{file}` enthalten, die dann in späteren `.replace()`-Schritten expandiert werden.
- Dadurch können untrusted Inputs in andere Prompt-Segmente “verschoben”/dupliziert werden (inkl. Bruch von Quotes/Backticks), was Prompt-Injection vereinfacht und Guardrails/Scope-Instruktionen unterwandern kann.

## Attack Scenario
1. Angreifer kontrolliert ein Eingabefeld, das in `target_text`/`scope_text`/`file` landet (z.B. Dateiname/Selection-Target oder ein Target-String aus Control-API), und platziert `{description}` oder `{file}` darin.
2. KYCO baut für eine Chain-Step den Base-Prompt via `Config::build_prompt()` (siehe `src/agent/chain/prompt.rs`).
3. Durch die sequenziellen `.replace()`-Aufrufe wird z.B. `{description}` innerhalb von `target_text`/`file` im letzten Schritt ersetzt und damit in einem anderen Prompt-Abschnitt injiziert (potenziell außerhalb der erwarteten Markdown-Quoting-Struktur).
4. Das Modell erhält einen manipulierten Prompt und kann (je nach Mode/Agent-Policy) zu unerwarteten File-Edits oder Tool-Calls bewegt werden.

## Suggested Fix
- Template-Rendering in *einem* Pass implementieren (echter Template-Parser), sodass nur Placeholder im ursprünglichen Template ersetzt werden und nicht in bereits eingesetzten Werten.
- Optional: `{...}` Sequenzen in untrusted Inputs escapen bzw. restriktiv erlauben (und Backticks/Newlines für Pfade/Ziele normalisieren), um Prompt-Injection über Dateinamen/Targets zu erschweren.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben
