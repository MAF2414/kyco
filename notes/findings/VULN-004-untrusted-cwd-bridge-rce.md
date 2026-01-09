# VULN-004: Untrusted `./bridge` directory leads to RCE on first run

## Summary
`BridgeProcess::spawn()` will use a `bridge/` directory from the current working directory if no installed bridge exists, then runs `npm install`/`npm run build` and executes `node dist/server.js` from that untrusted path.

## Severity
**CRITICAL** - Enables arbitrary code execution by placing a malicious `bridge/` folder in an untrusted repo and having a victim run KYCO for the first time (or without `~/.kyco/bridge` present).

## Location
src/agent/bridge/client/process.rs:28
src/agent/bridge/client/process.rs:158

## Code
```rust
// src/agent/bridge/client/process.rs
// If a bridge is already running (e.g., started externally), reuse it.
// ...
let bridge_dir = Self::find_bridge_dir()?;

// Check if node_modules exists, if not install dependencies.
let node_modules = bridge_dir.join("node_modules");
if !node_modules.exists() {
    // runs `npm ci` / `npm install` in `bridge_dir`
    let status = Command::new("npm")
        .arg("install")
        .current_dir(&bridge_dir)
        .status()?;
    // ...
}

// Build and then execute attacker-controlled JS
let mut child = Command::new("node")
    .arg("dist/server.js")
    .current_dir(&bridge_dir)
    .spawn()?;

// find_bridge_dir() fallback:
let cwd_bridge = PathBuf::from("bridge");
if cwd_bridge.exists() {
    return Ok(cwd_bridge);
}
```

## Impact
An attacker can achieve arbitrary code execution under the victim user account (via `npm` lifecycle scripts and/or `dist/server.js`), enabling secret theft, filesystem modification, persistence, and further compromise.

## Attack Scenario
1. Attacker publishes a repo containing `bridge/package.json` with a malicious `preinstall`/`install`/`postinstall` script (or a malicious `npm run build` script), plus optional `dist/server.js`.
2. Victim clones the repo and runs KYCO in that directory on a machine without `~/.kyco/bridge` installed yet (first run / cleaned home).
3. KYCO detects `./bridge` and runs `npm install`/`npm run build`, executing attacker scripts, then starts `node dist/server.js`.
4. Attacker code executes with the victim’s privileges.

## Suggested Fix
- Remove the `cwd_bridge` fallback entirely; only use `KYCO_BRIDGE_PATH`, an app-bundled bridge directory, or `~/.kyco/bridge`.
- If a “dev override” is needed, gate it behind an explicit CLI flag/config (e.g., `--bridge-path`) and/or only enable it in debug builds.
- Consider validating the bridge directory structure (expected files) and refusing to run `npm install` automatically from non-owned/untrusted paths.

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben
