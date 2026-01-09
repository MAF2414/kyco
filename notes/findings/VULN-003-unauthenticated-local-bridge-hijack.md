# VULN-003: Unauthenticated local bridge hijack (localhost impersonation)

## Summary
The bridge client connects to a fixed localhost HTTP endpoint without any authentication, and `BridgeProcess::spawn()` will reuse any server that responds to `/health`, enabling local impersonation to exfiltrate prompts/env/attachments and spoof streamed events.

## Severity
**MEDIUM** - A local attacker (or another user on the same machine) can transparently hijack the bridge channel to steal sensitive data and manipulate KYCO’s perceived agent output.

## Location
src/agent/bridge/client/mod.rs:37
src/agent/bridge/client/mod.rs:74
src/agent/bridge/client/process.rs:29

## Code
```rust
// src/agent/bridge/client/mod.rs
const DEFAULT_BRIDGE_URL: &str = "http://127.0.0.1:17432";

pub fn health_check(&self) -> Result<HealthResponse> {
    let url = format!("{}/health", self.base_url);
    let response: HealthResponse = self
        .control
        .get(&url)
        .call()
        .context("Failed to connect to bridge")?
        .into_json()
        .context("Failed to parse health response")?;
    Ok(response)
}
```

```rust
// src/agent/bridge/client/process.rs
// If a bridge is already running (e.g., started externally), reuse it.
if BridgeClient::new().health_check().is_ok() {
    return Ok(Self { child: None, running: Arc::new(AtomicBool::new(true)) });
}
```

## Impact
- Confidentiality breach: prompts, images (base64), working directory, and optional `env` passed to the bridge can be captured by a spoofed localhost service.
- Integrity loss: attacker-controlled NDJSON events can mislead the UI/runner about tool usage, outputs, and completion state.

## Attack Scenario
1. Attacker starts a local HTTP server binding `127.0.0.1:17432` and implements a minimal `/health` handler returning valid JSON.
2. Victim starts KYCO; `BridgeProcess::spawn()` sees `/health` succeed and reuses the attacker’s server.
3. Victim runs a job; KYCO sends `ClaudeQueryRequest`/`CodexQueryRequest` (prompt, cwd, optional `env`, images, etc.) to the attacker endpoint.
4. Attacker stores/exfiltrates the data and streams back crafted events to misrepresent what the agent did.

## Suggested Fix
- Add an authentication mechanism between KYCO and the bridge (e.g., random per-run token passed via env, sent as an `Authorization` header on every request, and validated by the bridge).
- Avoid fixed ports by default (pick a random free port or use a Unix domain socket with filesystem permissions).
- If “reuse externally started bridge” is required, require an explicit opt-in plus a shared secret, not just a successful unauthenticated `/health`.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben
