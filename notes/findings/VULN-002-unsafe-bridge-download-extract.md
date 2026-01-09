# VULN-002: Unsafe bridge download + tar extraction enables file write/RCE

## Summary
The bridge is fetched from GitHub “latest” via `curl` and extracted with `tar -xzf` without integrity verification or safe-path checks, allowing a malicious tarball to overwrite arbitrary files (path traversal) and/or plant code that KYCO executes.

## Severity
**HIGH** - Remote supply-chain compromise (or tampered release artifact) can yield arbitrary file writes and code execution when KYCO installs/starts the bridge.

## Location
src/agent/bridge/client/process.rs:169

## Code
```rust
// src/agent/bridge/client/process.rs
let download_url = format!(
    "https://github.com/{}/releases/latest/download/kyco-bridge.tar.gz",
    GITHUB_REPO
);

let output = Command::new("curl")
    .args(["-L", "-f", "-#", "-o", tarball_path.to_str().unwrap_or("kyco-bridge.tar.gz"), &download_url])
    .output()?;

let output = Command::new("tar")
    .args(["-xzf", tarball_path.to_str().unwrap_or("kyco-bridge.tar.gz"), "-C", kyco_dir.to_str().unwrap_or(".")])
    .output()?;
```

## Impact
If the downloaded tarball is malicious, it can:
- Write files outside `~/.kyco/` via `../` or absolute paths in tar entries (tarbomb/path traversal).
- Replace bridge code to be executed later (`node dist/server.js`), leading to arbitrary code execution.
- Overwrite user configuration files (e.g., shell profiles, SSH config) for persistence.

## Attack Scenario
1. Attacker compromises the GitHub release asset (or otherwise causes KYCO to download a malicious `kyco-bridge.tar.gz`).
2. Victim runs KYCO; `BridgeProcess::download_and_install_bridge()` downloads the tarball and extracts it with `tar -xzf` into `~/.kyco/`.
3. Malicious tar entries write outside the intended directory and/or install a trojan bridge.
4. KYCO subsequently runs the bridge (`node dist/server.js`), executing attacker code and/or the victim is persistently backdoored.

## Suggested Fix
- Pin a specific release version and verify integrity (e.g., SHA-256) before extraction; ideally verify a signature (Sigstore/Cosign or GPG).
- Replace shelling out to `tar` with a Rust tar/gzip implementation that validates each entry path is within the target directory.
- If using `tar`, enforce safer flags and explicitly reject absolute/parent paths (still recommend in-process validation over flags).

## Status
- [x] Verifiziert im Code
- [ ] PoC erstellt
- [ ] Report geschrieben

