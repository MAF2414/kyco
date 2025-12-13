# System-Prompts modifizieren

Lernen Sie, wie Sie Claudes Verhalten durch Modifizierung von System-Prompts mit drei Ansätzen anpassen können - Output-Stile, systemPrompt mit append und benutzerdefinierte System-Prompts.

---

System-Prompts definieren Claudes Verhalten, Fähigkeiten und Antwort-Stil. Das Claude Agent SDK bietet drei Möglichkeiten, System-Prompts anzupassen: die Verwendung von Output-Stilen (persistente, dateibasierte Konfigurationen), das Anhängen an Claude Codes Prompt oder die Verwendung eines vollständig benutzerdefinierten Prompts.

## System-Prompts verstehen

Ein System-Prompt ist der anfängliche Anweisungssatz, der bestimmt, wie sich Claude während einer Unterhaltung verhält.

<Note>
**Standardverhalten:** Das Agent SDK verwendet standardmäßig einen **leeren System-Prompt** für maximale Flexibilität. Um Claude Codes System-Prompt (Tool-Anweisungen, Code-Richtlinien usw.) zu verwenden, geben Sie `systemPrompt: { preset: "claude_code" }` in TypeScript oder `system_prompt="claude_code"` in Python an.
</Note>

Claude Codes System-Prompt umfasst:

- Tool-Nutzungsanweisungen und verfügbare Tools
- Code-Stil und Formatierungsrichtlinien
- Antwort-Ton und Ausführlichkeitseinstellungen
- Sicherheits- und Schutzanweisungen
- Kontext über das aktuelle Arbeitsverzeichnis und die Umgebung

## Modifikationsmethoden

### Methode 1: Output-Stile (persistente Konfigurationen)

Output-Stile sind gespeicherte Konfigurationen, die Claudes System-Prompt modifizieren. Sie werden als Markdown-Dateien gespeichert und können sitzungs- und projektübergreifend wiederverwendet werden.

#### Einen Output-Stil erstellen

<CodeGroup>

```typescript TypeScript
import { writeFile, mkdir } from 'fs/promises'
import { join } from 'path'
import { homedir } from 'os'

async function createOutputStyle(name: string, description: string, prompt: string) {
  // Benutzerebene: ~/.claude/output-styles
  // Projektebene: .claude/output-styles
  const outputStylesDir = join(homedir(), '.claude', 'output-styles')
  
  await mkdir(outputStylesDir, { recursive: true })
  
  const content = `---
name: ${name}
description: ${description}
---

${prompt}`
  
  const filePath = join(outputStylesDir, `${name.toLowerCase().replace(/\s+/g, '-')}.md`)
  await writeFile(filePath, content, 'utf-8')
}

// Beispiel: Einen Code-Review-Spezialisten erstellen
await createOutputStyle(
  'Code Reviewer',
  'Gründlicher Code-Review-Assistent',
  `Sie sind ein Experte für Code-Reviews.

Für jede Code-Einreichung:
1. Prüfen Sie auf Bugs und Sicherheitsprobleme
2. Bewerten Sie die Performance
3. Schlagen Sie Verbesserungen vor
4. Bewerten Sie die Code-Qualität (1-10)`
)
```

```python Python
from pathlib import Path

async def create_output_style(name: str, description: str, prompt: str):
    # Benutzerebene: ~/.claude/output-styles
    # Projektebene: .claude/output-styles
    output_styles_dir = Path.home() / '.claude' / 'output-styles'
    
    output_styles_dir.mkdir(parents=True, exist_ok=True)
    
    content = f"""---
name: {name}
description: {description}
---

{prompt}"""
    
    file_name = name.lower().replace(' ', '-') + '.md'
    file_path = output_styles_dir / file_name
    file_path.write_text(content, encoding='utf-8')

# Beispiel: Einen Code-Review-Spezialisten erstellen
await create_output_style(
    'Code Reviewer',
    'Gründlicher Code-Review-Assistent',
    """Sie sind ein Experte für Code-Reviews.

Für jede Code-Einreichung:
1. Prüfen Sie auf Bugs und Sicherheitsprobleme
2. Bewerten Sie die Performance
3. Schlagen Sie Verbesserungen vor
4. Bewerten Sie die Code-Qualität (1-10)"""
)
```

</CodeGroup>

#### Output-Stile verwenden

Nach der Erstellung aktivieren Sie Output-Stile über:
- **CLI**: `/output-style [stil-name]`
- **Einstellungen**: `.claude/settings.local.json`
- **Neu erstellen**: `/output-style:new [beschreibung]`

### Methode 2: `systemPrompt` mit append verwenden

Sie können das Claude Code Preset mit einer `append`-Eigenschaft verwenden, um Ihre benutzerdefinierten Anweisungen hinzuzufügen, während Sie alle eingebauten Funktionen beibehalten.

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

const messages = []

for await (const message of query({
  prompt: "Hilf mir, eine Python-Funktion zur Berechnung von Fibonacci-Zahlen zu schreiben",
  options: {
    systemPrompt: {
      type: "preset",
      preset: "claude_code",
      append: "Fügen Sie immer detaillierte Docstrings und Type Hints in Python-Code ein."
    }
  }
})) {
  messages.push(message)
  if (message.type === 'assistant') {
    console.log(message.message.content)
  }
}
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

messages = []

async for message in query(
    prompt="Hilf mir, eine Python-Funktion zur Berechnung von Fibonacci-Zahlen zu schreiben",
    options=ClaudeAgentOptions(
        system_prompt={
            "type": "preset",
            "preset": "claude_code",
            "append": "Fügen Sie immer detaillierte Docstrings und Type Hints in Python-Code ein."
        }
    )
):
    messages.append(message)
    if message.type == 'assistant':
        print(message.message.content)
```

</CodeGroup>

### Methode 3: Benutzerdefinierte System-Prompts

Sie können einen benutzerdefinierten String als `systemPrompt` bereitstellen, um den Standard vollständig durch Ihre eigenen Anweisungen zu ersetzen.

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

const customPrompt = `Sie sind ein Python-Coding-Spezialist.
Befolgen Sie diese Richtlinien:
- Schreiben Sie sauberen, gut dokumentierten Code
- Verwenden Sie Type Hints für alle Funktionen
- Fügen Sie umfassende Docstrings ein
- Bevorzugen Sie funktionale Programmiermuster, wenn angemessen
- Erklären Sie immer Ihre Code-Entscheidungen`

const messages = []

for await (const message of query({
  prompt: "Erstelle eine Datenverarbeitungs-Pipeline",
  options: {
    systemPrompt: customPrompt
  }
})) {
  messages.push(message)
  if (message.type === 'assistant') {
    console.log(message.message.content)
  }
}
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

custom_prompt = """Sie sind ein Python-Coding-Spezialist.
Befolgen Sie diese Richtlinien:
- Schreiben Sie sauberen, gut dokumentierten Code
- Verwenden Sie Type Hints für alle Funktionen
- Fügen Sie umfassende Docstrings ein
- Bevorzugen Sie funktionale Programmiermuster, wenn angemessen
- Erklären Sie immer Ihre Code-Entscheidungen"""

messages = []

async for message in query(
    prompt="Erstelle eine Datenverarbeitungs-Pipeline",
    options=ClaudeAgentOptions(
        system_prompt=custom_prompt
    )
):
    messages.append(message)
    if message.type == 'assistant':
        print(message.message.content)
```

</CodeGroup>

## Vergleich aller drei Ansätze

| Feature | Output-Stile | `systemPrompt` mit append | Benutzerdefinierter `systemPrompt` |
|---|---|---|---|
| **Persistenz** | ✅ Als Dateien gespeichert | ❌ Nur Sitzung | ❌ Nur Sitzung |
| **Wiederverwendbarkeit** | ✅ Projektübergreifend | ❌ Code-Duplikation | ❌ Code-Duplikation |
| **Verwaltung** | ✅ CLI + Dateien | ⚠️ Im Code | ⚠️ Im Code |
| **Standard-Tools** | ✅ Erhalten | ✅ Erhalten | ❌ Verloren (außer wenn eingeschlossen) |
| **Eingebaute Sicherheit** | ✅ Beibehalten | ✅ Beibehalten | ❌ Muss hinzugefügt werden |
| **Umgebungskontext** | ✅ Automatisch | ✅ Automatisch | ❌ Muss bereitgestellt werden |
| **Anpassungsgrad** | ⚠️ Standard ersetzen | ⚠️ Nur Ergänzungen | ✅ Vollständige Kontrolle |
| **Versionskontrolle** | ✅ Ja | ✅ Mit Code | ✅ Mit Code |
| **Entdeckung** | ✅ `/output-style` | ❌ Nicht entdeckbar | ❌ Nicht entdeckbar |

**Hinweis:** "Mit append" bedeutet die Verwendung von `systemPrompt: { type: "preset", preset: "claude_code", append: "..." }` in TypeScript oder `system_prompt={"type": "preset", "preset": "claude_code", "append": "..."}` in Python.

## Anwendungsfälle und bewährte Praktiken

### Wann Output-Stile verwendet werden sollten

**Am besten für:**
- Persistente Verhaltensänderungen über Sitzungen hinweg
- Team-geteilte Konfigurationen
- Spezialisierte Assistenten (Code-Reviewer, Datenwissenschaftler, DevOps)
- Komplexe Prompt-Modifikationen, die Versionierung benötigen

**Beispiele:**
- Erstellen eines dedizierten SQL-Optimierungs-Assistenten
- Aufbau eines sicherheitsfokussierten Code-Reviewers
- Entwicklung eines Lehr-Assistenten mit spezifischer Pädagogik

### Wann `systemPrompt` mit append verwendet werden sollte

**Am besten für:**
- Hinzufügen spezifischer Coding-Standards oder Präferenzen
- Anpassen der Output-Formatierung
- Hinzufügen domänenspezifischen Wissens
- Modifizieren der Antwort-Ausführlichkeit
- Verbesserung von Claude Codes Standardverhalten ohne Verlust der Tool-Anweisungen

### Wann benutzerdefinierte `systemPrompt` verwendet werden sollte

**Am besten für:**
- Vollständige Kontrolle über Claudes Verhalten
- Spezialisierte Einzelsitzungs-Aufgaben
- Testen neuer Prompt-Strategien
- Situationen, in denen Standard-Tools nicht benötigt werden
- Aufbau spezialisierter Agenten mit einzigartigem Verhalten

## Ansätze kombinieren

Sie können diese Methoden für maximale Flexibilität kombinieren:

### Beispiel: Output-Stil mit sitzungsspezifischen Ergänzungen

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

// Angenommen, "Code Reviewer" Output-Stil ist aktiv (über /output-style)
// Sitzungsspezifische Schwerpunktbereiche hinzufügen
const messages = []

for await (const message of query({
  prompt: "Überprüfe dieses Authentifizierungsmodul",
  options: {
    systemPrompt: {
      type: "preset",
      preset: "claude_code",
      append: `
        Für diese Überprüfung priorisieren Sie:
        - OAuth 2.0 Compliance
        - Token-Speicher-Sicherheit
        - Session-Management
      `
    }
  }
})) {
  messages.push(message)
}
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

# Angenommen, "Code Reviewer" Output-Stil ist aktiv (über /output-style)
# Sitzungsspezifische Schwerpunktbereiche hinzufügen
messages = []

async for message in query(
    prompt="Überprüfe dieses Authentifizierungsmodul",
    options=ClaudeAgentOptions(
        system_prompt={
            "type": "preset",
            "preset": "claude_code",
            "append": """
            Für diese Überprüfung priorisieren Sie:
            - OAuth 2.0 Compliance
            - Token-Speicher-Sicherheit
            - Session-Management
            """
        }
    )
):
    messages.append(message)
```

</CodeGroup>

## Siehe auch

- [Output-Stile](https://code.claude.com/docs/output-styles) - Vollständige Output-Stile-Dokumentation
- [TypeScript SDK Leitfaden](/docs/de/agent-sdk/typescript) - Vollständiger SDK-Nutzungsleitfaden
- [TypeScript SDK Referenz](https://code.claude.com/docs/typescript-sdk-reference) - Vollständige API-Dokumentation  
- [Konfigurationsleitfaden](https://code.claude.com/docs/configuration) - Allgemeine Konfigurationsoptionen