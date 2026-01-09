# VULN-001: Unbounded Bridge/Agent Output Causes Memory DoS

## Summary
Mehrere Codepfade übernehmen untrusted Bridge/Agent-Output ohne Größenlimit in `Job`-State und Logs. Ein bösartiger/kompromittierter Bridge-Server (oder manipulierte Agent-Outputs) kann dadurch Speicher erschöpfen und die Anwendung zum Absturz bringen.

## Severity
**MEDIUM** - Remote/IPC-input kann zu Prozess-crash/Freeze (DoS) führen; die Limits begrenzen nur die Anzahl der Log-Events, nicht deren Payload-Größe, und der NDJSON-Reader kann einzelne Zeilen unbounded allozieren.

## Location
src/domain/job/impls.rs:105-114
src/domain/job/impls.rs:130-133
src/agent/bridge/client/stream.rs:23-56
src/agent/bridge/adapter/codex.rs:186-189
src/agent/bridge/adapter/claude.rs:242-245

## Code
```rust
// src/domain/job/impls.rs
pub fn add_log_event(&mut self, event: LogEvent) {
    self.log_events.push(event);
    if self.log_events.len() > MAX_JOB_LOG_EVENTS {
        let excess = self.log_events.len() - MAX_JOB_LOG_EVENTS;
        self.log_events.drain(0..excess);
    }
    self.updated_at = Utc::now();
}

pub fn parse_result(&mut self, output: &str) {
    self.result = JobResult::parse(output);
}
```

```rust
// src/agent/bridge/client/stream.rs
match self.reader.read_line(&mut self.buffer) { /* ... */ }
// ...
Err(e) => Some(Err(anyhow::anyhow!(
    "Failed to parse event: {} (line: {})",
    e,
    trimmed
))),
```

```rust
// src/agent/bridge/adapter/codex.rs
BridgeEvent::ToolResult { output, files_changed, .. } => {
    /* ... */
    let _ = event_tx.send(LogEvent::tool_output("tool", output).for_job(job_id)).await;
}
```

## Impact
- Memory Exhaustion / Crash: Ein einzelnes sehr großes NDJSON-Event (oder sehr große Tool-Outputs) kann den Heap aufblasen (Reader-Buffer + JSON parse + Logging + Job-Storage).
- UI/Executor Freeze: Große Strings werden mehrfach geklont/verschoben (Event→LogEvent→Job.log_events / `full_response`) und können Rendering/Serialization massiv verlangsamen.

## Attack Scenario
1. Angreifer betreibt/kompromittiert den lokalen Bridge-Endpunkt (Default `http://127.0.0.1:17432`) oder erzwingt eine bösartige Bridge-Konfiguration.
2. Der Bridge-Server streamt NDJSON mit einer extrem großen Zeile (z.B. ein `ToolResult` mit `output` im zweistelligen/mehrstelligen MB-Bereich) oder mit invalid JSON, sodass der Error-Path die komplette Zeile in die Fehlermeldung interpoliert.
3. `EventStream::read_line()` allokiert die komplette Zeile in `buffer`; anschließend wird der Payload in LogEvents/Job-State übernommen (bis zu 200 Events), wodurch der Prozess OOMt oder stark degradiert und abstürzt.

## Suggested Fix
- Hard-Limits einführen:
  - Max bytes pro NDJSON-Line (Reader): Abort/Drop wenn überschritten (z.B. `read_until(b'\n', ...)` + cap).
  - Max bytes pro `LogEvent.summary`, `LogEvent.content`, `tool_args`-JSON und `Job.full_response` (truncate mit klarer Markierung `"[truncated]"`).
  - YAML/JSON Result Parsing nur auf einem begrenzten Slice (z.B. letzte N KB) oder nur auf dem extrahierten Result-Block; zusätzlich YAML-Alias/Depth-Limits (sofern crate unterstützt) bzw. defensive Parser-Konfiguration.
- Error-Message-Härtung: Beim Parse-Error nicht die komplette `trimmed`-Line loggen, sondern nur Prefix+Hash/Len.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben
