<p align="center">
  <img src="assets/Logo.png" alt="KYCo Logo" width="200">
</p>

# KYCo - Know Your Codebase

Hey! 👋 Kennst du das? Du startest nen AI-Agent und 2 Stunden später hat der halbe Codebase umgebaut. KYCo macht das anders:

**Du wählst Code aus → sagst was passieren soll → reviewst den Diff → fertig.**

Keine endlosen Agent-Sessions mehr. Du bleibst in Control.

## Was kann das Ding?

- 🎯 **Gezielte Changes** - Markier Code, wähl nen Mode, krieg genau die Änderung die du brauchst
- 🤖 **Multi-Agent** - Claude und Codex parallel laufen lassen? Kein Problem
- 🎤 **Voice Input** - Einfach reinsprechen was du willst (Whisper macht die Transkription)
- 👀 **Du entscheidest** - Jeder Diff wird dir gezeigt, du sagst Accept oder Reject

## Installation

### Was du brauchst

- Node.js >= 18 (für den SDK Bridge Server)
- Claude CLI oder Codex CLI

### macOS

```bash
# M1/M2/M3/M4
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-arm64

# Intel
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-macos-x64

chmod +x kyco
sudo mv kyco /usr/local/bin/

# Falls Gatekeeper meckert:
xattr -d com.apple.quarantine /usr/local/bin/kyco
```

### Linux

```bash
curl -L -o kyco https://github.com/MAF2414/kyco/releases/latest/download/kyco-linux-x64
chmod +x kyco
sudo mv kyco /usr/local/bin/
```

### Windows

Hol dir `kyco-windows-x64.exe` von den [Releases](https://github.com/MAF2414/kyco/releases/latest) und pack's in deinen PATH.

### Selber bauen

```bash
git clone https://github.com/MAF2414/kyco.git
cd kyco
cargo install --path .
```

Brauchst Rust 1.75+

### IDE Extensions

**VS Code:**
```bash
# vsix von Releases holen, dann:
code --install-extension kyco-vscode.vsix
```

**JetBrains:** Settings → Plugins → ⚙️ → Install from Disk → die zip auswählen

## Los geht's

```bash
kyco init    # Config erstellen
kyco         # GUI starten
```

Dann in der IDE: Code markieren → `Cmd+Alt+Y` (Mac) bzw. `Ctrl+Alt+Y` → Mode wählen → Enter → Diff reviewen → Done ✅

## Die wichtigsten Modes

| Mode | Shortcut | Was macht's? |
|------|----------|--------------|
| `chat` | `c` | Einfach quatschen über den Code |
| `implement` | `i` | Neues Feature bauen |
| `review` | `r` | Code checken (read-only) |
| `fix` | `f` | Bug fixen |
| `refactor` | `ref` | Aufräumen ohne Funktionsänderung |
| `test` | `t` | Tests schreiben |
| `plan` | `p` | Plan erstellen (read-only) |

## Chains - mehrere Modes hintereinander

Willst du erst reviewen und dann fixen? Dafür gibt's Chains:

```toml
[chain."review+fix"]
description = "Erst checken, dann fixen"
steps = [
    { mode = "review" },
    { mode = "fix", trigger_on = ["issues_found"] },
]
```

Eingebaute Chains:
- `refactor-safe` → Review → Refactor → Test
- `implement-and-test` → Implement → Test
- `quality-gate` → Review → Security → Types → Coverage

## Config

Liegt in `~/.kyco/config.toml` (global) oder `.kyco/config.toml` (pro Projekt):

```toml
[settings]
max_concurrent_jobs = 4      # Wieviele Jobs parallel
auto_run = true              # Jobs direkt starten
use_worktree = false         # Jobs in Git Worktrees isolieren

[agent.claude]
aliases = ["c", "cl"]
sdk = "claude"

[agent.codex]
aliases = ["x", "cx"]
sdk = "codex"

# Eigene Modes definieren:
[mode.cleanup]
aliases = ["cu"]
prompt = "Räum den Code auf, entfern toten Code"
```

## Shortcuts

### In der IDE

| Was | Mac | Windows/Linux |
|-----|-----|---------------|
| Code schicken | `Cmd+Alt+Y` | `Ctrl+Alt+Y` |
| Grep & schicken | `Cmd+Alt+Shift+G` | `Ctrl+Alt+Shift+G` |

### In KYCo

| Was | Taste |
|-----|-------|
| Job starten | `Enter` |
| Mit Worktree | `Shift+Enter` |
| Voice | `Cmd+D` |
| Popup schließen | `Esc` |
| Jobs navigieren | `j`/`k` oder Pfeiltasten |
| Auto-run toggle | `Shift+A` |

### Voice Hotkey (überall!)

| Was | Mac | Windows/Linux |
|-----|-----|---------------|
| Diktat starten/stoppen | `Cmd+Shift+V` | `Ctrl+Shift+V` |

Das Coole: Funktioniert von überall! Auch wenn Claude Code im Terminal läuft. Einmal drücken = Recording, nochmal drücken = fertig, Text wird automatisch eingefügt.

## CLI Commands

```bash
kyco                    # GUI starten
kyco init               # Config erstellen
kyco status             # Jobs anzeigen
kyco job start --file src/foo.rs --mode fix --prompt "Fix den Bug"
kyco job wait 1         # Warten bis Job fertig
kyco job output 1       # Output holen
kyco job continue 1 --prompt "Mach noch Tests dazu"
kyco job abort 1        # Job abbrechen
```

## Orchestrator Mode

Du kannst nen externen Agent (Claude Code / Codex) KYCo steuern lassen:

1. KYCo GUI starten (damit du alles siehst)
2. In nem zweiten Terminal den Agent starten
3. Der Agent ruft dann `kyco job ...` Commands auf

Gibt auch nen Orchestrator-Button in der Statusbar der direkt ne Claude/Codex Session in Terminal.app startet.

## Voice Input

KYCo nutzt Whisper für Speech-to-Text. Dependencies werden beim ersten Mal automatisch installiert.

**Im Popup:** Mikrofon-Button klicken oder `Cmd+D` → sprechen → Enter

**Global (in jeder App):** `Cmd+Shift+V` → sprechen → nochmal drücken → Text wird eingefügt

## Support

Wenn dir KYCo hilft, freu ich mich über nen [Sponsor](https://github.com/sponsors/MAF2414) ☕

## Lizenz

[Business Source License 1.1](LICENSE)

Kannst du frei nutzen, auch produktiv - solange du's nicht als Hosted Service anbietest oder als Konkurrenzprodukt verkaufst. Ab 2029 wird's Apache 2.0.

Fragen? [GitHub Issues](https://github.com/MAF2414/kyco/issues) oder einfach melden 👋
