# Slash-Befehle im SDK

Erfahren Sie, wie Sie Slash-Befehle verwenden, um Claude Code-Sitzungen über das SDK zu steuern

---

Slash-Befehle bieten eine Möglichkeit, Claude Code-Sitzungen mit speziellen Befehlen zu steuern, die mit `/` beginnen. Diese Befehle können über das SDK gesendet werden, um Aktionen wie das Löschen des Gesprächsverlaufs, das Komprimieren von Nachrichten oder das Abrufen von Hilfe durchzuführen.

## Verfügbare Slash-Befehle entdecken

Das Claude Agent SDK stellt Informationen über verfügbare Slash-Befehle in der Systeminitialisierungsnachricht bereit. Greifen Sie auf diese Informationen zu, wenn Ihre Sitzung startet:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Hallo Claude",
  options: { maxTurns: 1 }
})) {
  if (message.type === "system" && message.subtype === "init") {
    console.log("Verfügbare Slash-Befehle:", message.slash_commands);
    // Beispielausgabe: ["/compact", "/clear", "/help"]
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    async for message in query(
        prompt="Hallo Claude",
        options={"max_turns": 1}
    ):
        if message.type == "system" and message.subtype == "init":
            print("Verfügbare Slash-Befehle:", message.slash_commands)
            # Beispielausgabe: ["/compact", "/clear", "/help"]

asyncio.run(main())
```

</CodeGroup>

## Slash-Befehle senden

Senden Sie Slash-Befehle, indem Sie sie in Ihren Prompt-String einschließen, genau wie normalen Text:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Einen Slash-Befehl senden
for await (const message of query({
  prompt: "/compact",
  options: { maxTurns: 1 }
})) {
  if (message.type === "result") {
    console.log("Befehl ausgeführt:", message.result);
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    # Einen Slash-Befehl senden
    async for message in query(
        prompt="/compact",
        options={"max_turns": 1}
    ):
        if message.type == "result":
            print("Befehl ausgeführt:", message.result)

asyncio.run(main())
```

</CodeGroup>

## Häufige Slash-Befehle

### `/compact` - Gesprächsverlauf komprimieren

Der `/compact`-Befehl reduziert die Größe Ihres Gesprächsverlaufs, indem er ältere Nachrichten zusammenfasst und dabei wichtigen Kontext bewahrt:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "/compact",
  options: { maxTurns: 1 }
})) {
  if (message.type === "system" && message.subtype === "compact_boundary") {
    console.log("Komprimierung abgeschlossen");
    console.log("Token vor Komprimierung:", message.compact_metadata.pre_tokens);
    console.log("Auslöser:", message.compact_metadata.trigger);
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    async for message in query(
        prompt="/compact",
        options={"max_turns": 1}
    ):
        if (message.type == "system" and 
            message.subtype == "compact_boundary"):
            print("Komprimierung abgeschlossen")
            print("Token vor Komprimierung:", 
                  message.compact_metadata.pre_tokens)
            print("Auslöser:", message.compact_metadata.trigger)

asyncio.run(main())
```

</CodeGroup>

### `/clear` - Gespräch löschen

Der `/clear`-Befehl startet ein neues Gespräch, indem er den gesamten vorherigen Verlauf löscht:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Gespräch löschen und neu starten
for await (const message of query({
  prompt: "/clear",
  options: { maxTurns: 1 }
})) {
  if (message.type === "system" && message.subtype === "init") {
    console.log("Gespräch gelöscht, neue Sitzung gestartet");
    console.log("Sitzungs-ID:", message.session_id);
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    # Gespräch löschen und neu starten
    async for message in query(
        prompt="/clear",
        options={"max_turns": 1}
    ):
        if message.type == "system" and message.subtype == "init":
            print("Gespräch gelöscht, neue Sitzung gestartet")
            print("Sitzungs-ID:", message.session_id)

asyncio.run(main())
```

</CodeGroup>

## Benutzerdefinierte Slash-Befehle erstellen

Zusätzlich zur Verwendung eingebauter Slash-Befehle können Sie Ihre eigenen benutzerdefinierten Befehle erstellen, die über das SDK verfügbar sind. Benutzerdefinierte Befehle werden als Markdown-Dateien in bestimmten Verzeichnissen definiert, ähnlich wie Subagenten konfiguriert werden.

### Dateispeicherorte

Benutzerdefinierte Slash-Befehle werden in bestimmten Verzeichnissen basierend auf ihrem Geltungsbereich gespeichert:

- **Projektbefehle**: `.claude/commands/` - Nur im aktuellen Projekt verfügbar
- **Persönliche Befehle**: `~/.claude/commands/` - In allen Ihren Projekten verfügbar

### Dateiformat

Jeder benutzerdefinierte Befehl ist eine Markdown-Datei, bei der:
- Der Dateiname (ohne `.md`-Erweiterung) zum Befehlsnamen wird
- Der Dateiinhalt definiert, was der Befehl tut
- Optionale YAML-Frontmatter bietet Konfiguration

#### Grundlegendes Beispiel

Erstellen Sie `.claude/commands/refactor.md`:

```markdown
Refaktorisieren Sie den ausgewählten Code, um Lesbarkeit und Wartbarkeit zu verbessern.
Konzentrieren Sie sich auf Clean-Code-Prinzipien und bewährte Praktiken.
```

Dies erstellt den `/refactor`-Befehl, den Sie über das SDK verwenden können.

#### Mit Frontmatter

Erstellen Sie `.claude/commands/security-check.md`:

```markdown
---
allowed-tools: Read, Grep, Glob
description: Sicherheitsvulnerabilitätsscan durchführen
model: claude-3-5-sonnet-20241022
---

Analysieren Sie die Codebasis auf Sicherheitsvulnerabilitäten einschließlich:
- SQL-Injection-Risiken
- XSS-Vulnerabilitäten
- Exponierte Anmeldedaten
- Unsichere Konfigurationen
```

### Benutzerdefinierte Befehle im SDK verwenden

Sobald sie im Dateisystem definiert sind, sind benutzerdefinierte Befehle automatisch über das SDK verfügbar:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Einen benutzerdefinierten Befehl verwenden
for await (const message of query({
  prompt: "/refactor src/auth/login.ts",
  options: { maxTurns: 3 }
})) {
  if (message.type === "assistant") {
    console.log("Refaktorisierungsvorschläge:", message.message);
  }
}

// Benutzerdefinierte Befehle erscheinen in der slash_commands-Liste
for await (const message of query({
  prompt: "Hallo",
  options: { maxTurns: 1 }
})) {
  if (message.type === "system" && message.subtype === "init") {
    // Wird sowohl eingebaute als auch benutzerdefinierte Befehle enthalten
    console.log("Verfügbare Befehle:", message.slash_commands);
    // Beispiel: ["/compact", "/clear", "/help", "/refactor", "/security-check"]
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    # Einen benutzerdefinierten Befehl verwenden
    async for message in query(
        prompt="/refactor src/auth/login.py",
        options={"max_turns": 3}
    ):
        if message.type == "assistant":
            print("Refaktorisierungsvorschläge:", message.message)
    
    # Benutzerdefinierte Befehle erscheinen in der slash_commands-Liste
    async for message in query(
        prompt="Hallo",
        options={"max_turns": 1}
    ):
        if message.type == "system" and message.subtype == "init":
            # Wird sowohl eingebaute als auch benutzerdefinierte Befehle enthalten
            print("Verfügbare Befehle:", message.slash_commands)
            # Beispiel: ["/compact", "/clear", "/help", "/refactor", "/security-check"]

asyncio.run(main())
```

</CodeGroup>

### Erweiterte Funktionen

#### Argumente und Platzhalter

Benutzerdefinierte Befehle unterstützen dynamische Argumente mit Platzhaltern:

Erstellen Sie `.claude/commands/fix-issue.md`:

```markdown
---
argument-hint: [issue-nummer] [priorität]
description: Ein GitHub-Issue beheben
---

Beheben Sie Issue #$1 mit Priorität $2.
Überprüfen Sie die Issue-Beschreibung und implementieren Sie die notwendigen Änderungen.
```

Verwendung im SDK:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Argumente an benutzerdefinierten Befehl übergeben
for await (const message of query({
  prompt: "/fix-issue 123 high",
  options: { maxTurns: 5 }
})) {
  // Befehl wird mit $1="123" und $2="high" verarbeitet
  if (message.type === "result") {
    console.log("Issue behoben:", message.result);
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    # Argumente an benutzerdefinierten Befehl übergeben
    async for message in query(
        prompt="/fix-issue 123 high",
        options={"max_turns": 5}
    ):
        # Befehl wird mit $1="123" und $2="high" verarbeitet
        if message.type == "result":
            print("Issue behoben:", message.result)

asyncio.run(main())
```

</CodeGroup>

#### Bash-Befehlsausführung

Benutzerdefinierte Befehle können Bash-Befehle ausführen und deren Ausgabe einschließen:

Erstellen Sie `.claude/commands/git-commit.md`:

```markdown
---
allowed-tools: Bash(git add:*), Bash(git status:*), Bash(git commit:*)
description: Einen Git-Commit erstellen
---

## Kontext

- Aktueller Status: !`git status`
- Aktueller Diff: !`git diff HEAD`

## Aufgabe

Erstellen Sie einen Git-Commit mit angemessener Nachricht basierend auf den Änderungen.
```

#### Dateireferenzen

Schließen Sie Dateiinhalte mit dem `@`-Präfix ein:

Erstellen Sie `.claude/commands/review-config.md`:

```markdown
---
description: Konfigurationsdateien überprüfen
---

Überprüfen Sie die folgenden Konfigurationsdateien auf Probleme:
- Paket-Konfiguration: @package.json
- TypeScript-Konfiguration: @tsconfig.json
- Umgebungs-Konfiguration: @.env

Prüfen Sie auf Sicherheitsprobleme, veraltete Abhängigkeiten und Fehlkonfigurationen.
```

### Organisation mit Namensräumen

Organisieren Sie Befehle in Unterverzeichnissen für bessere Struktur:

```bash
.claude/commands/
├── frontend/
│   ├── component.md      # Erstellt /component (project:frontend)
│   └── style-check.md     # Erstellt /style-check (project:frontend)
├── backend/
│   ├── api-test.md        # Erstellt /api-test (project:backend)
│   └── db-migrate.md      # Erstellt /db-migrate (project:backend)
└── review.md              # Erstellt /review (project)
```

Das Unterverzeichnis erscheint in der Befehlsbeschreibung, beeinflusst aber nicht den Befehlsnamen selbst.

### Praktische Beispiele

#### Code-Review-Befehl

Erstellen Sie `.claude/commands/code-review.md`:

```markdown
---
allowed-tools: Read, Grep, Glob, Bash(git diff:*)
description: Umfassendes Code-Review
---

## Geänderte Dateien
!`git diff --name-only HEAD~1`

## Detaillierte Änderungen
!`git diff HEAD~1`

## Review-Checkliste

Überprüfen Sie die obigen Änderungen auf:
1. Codequalität und Lesbarkeit
2. Sicherheitsvulnerabilitäten
3. Performance-Auswirkungen
4. Testabdeckung
5. Dokumentationsvollständigkeit

Geben Sie spezifisches, umsetzbares Feedback organisiert nach Priorität.
```

#### Test-Runner-Befehl

Erstellen Sie `.claude/commands/test.md`:

```markdown
---
allowed-tools: Bash, Read, Edit
argument-hint: [test-muster]
description: Tests mit optionalem Muster ausführen
---

Tests ausführen, die dem Muster entsprechen: $ARGUMENTS

1. Test-Framework erkennen (Jest, pytest, etc.)
2. Tests mit dem bereitgestellten Muster ausführen
3. Falls Tests fehlschlagen, analysieren und beheben
4. Erneut ausführen, um Korrekturen zu verifizieren
```

Verwenden Sie diese Befehle über das SDK:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Code-Review ausführen
for await (const message of query({
  prompt: "/code-review",
  options: { maxTurns: 3 }
})) {
  // Review-Feedback verarbeiten
}

// Spezifische Tests ausführen
for await (const message of query({
  prompt: "/test auth",
  options: { maxTurns: 5 }
})) {
  // Testergebnisse behandeln
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    # Code-Review ausführen
    async for message in query(
        prompt="/code-review",
        options={"max_turns": 3}
    ):
        # Review-Feedback verarbeiten
        pass
    
    # Spezifische Tests ausführen
    async for message in query(
        prompt="/test auth",
        options={"max_turns": 5}
    ):
        # Testergebnisse behandeln
        pass

asyncio.run(main())
```

</CodeGroup>

## Siehe auch

- [Slash-Befehle](https://code.claude.com/docs/slash-commands) - Vollständige Slash-Befehl-Dokumentation
- [Subagenten im SDK](/docs/de/agent-sdk/subagents) - Ähnliche dateisystembasierte Konfiguration für Subagenten
- [TypeScript SDK-Referenz](https://code.claude.com/docs/typescript-sdk-reference) - Vollständige API-Dokumentation
- [SDK-Übersicht](/docs/de/agent-sdk/overview) - Allgemeine SDK-Konzepte
- [CLI-Referenz](https://code.claude.com/docs/cli-reference) - Befehlszeilenschnittstelle