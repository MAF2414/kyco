# Agent Skills im SDK

Erweitern Sie Claude mit spezialisierten Fähigkeiten mithilfe von Agent Skills im Claude Agent SDK

---

## Übersicht

Agent Skills erweitern Claude mit spezialisierten Fähigkeiten, die Claude autonom aufruft, wenn relevant. Skills werden als `SKILL.md`-Dateien verpackt, die Anweisungen, Beschreibungen und optionale unterstützende Ressourcen enthalten.

Umfassende Informationen zu Skills, einschließlich Vorteile, Architektur und Authoring-Richtlinien, finden Sie in der [Agent Skills-Übersicht](/docs/de/agents-and-tools/agent-skills/overview).

## Wie Skills mit dem SDK funktionieren

Bei Verwendung des Claude Agent SDK sind Skills:

1. **Als Dateisystem-Artefakte definiert**: Erstellt als `SKILL.md`-Dateien in spezifischen Verzeichnissen (`.claude/skills/`)
2. **Automatisch erkannt**: Skill-Metadaten beim Start aus Benutzer- und Projektverzeichnissen geladen; vollständiger Inhalt wird geladen, wenn ausgelöst
3. **Modell-aufgerufen**: Claude wählt autonom aus, wann sie basierend auf dem Kontext verwendet werden
4. **Aktiviert über allowed_tools**: Fügen Sie `"Skill"` zu Ihrem `allowed_tools` hinzu, um Skills zu aktivieren

Im Gegensatz zu Subagenten (die programmatisch definiert werden können) müssen Skills als Dateisystem-Artefakte erstellt werden. Das SDK bietet keine programmatische API zum Registrieren von Skills. In KYCo können Sie Subagenten in `.kyco/config.toml` unter `[agent.claude.agents]` definieren.

## Verwenden von Skills mit dem SDK

Um Skills mit dem SDK zu verwenden, fügen Sie `"Skill"` in Ihre `allowed_tools`-Konfiguration ein. Nach der Aktivierung entdeckt Claude automatisch Skills aus dem `.claude/skills/`-Verzeichnis Ihres Projekts und ruft sie auf, wenn sie für die Anfrage des Benutzers relevant sind.

<CodeGroup>

```python Python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions

async def main():
    options = ClaudeAgentOptions(
        cwd="/path/to/project",  # Project with .claude/skills/
        allowed_tools=["Skill", "Read", "Write", "Bash"]  # Enable Skill tool
    )

    async for message in query(
        prompt="Help me process this PDF document",
        options=options
    ):
        print(message)

asyncio.run(main())
```

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Help me process this PDF document",
  options: {
    cwd: "/path/to/project",  // Project with .claude/skills/
    allowedTools: ["Skill", "Read", "Write", "Bash"]  // Enable Skill tool
})) {
  console.log(message);
}
```

</CodeGroup>

## Skill-Standorte

Skills werden automatisch aus designierten Dateisystem-Verzeichnissen geladen:

- **Project Skills** (`.claude/skills/`): Mit Ihrem Team über git geteilt
- **User Skills** (`~/.claude/skills/`): Persönliche Skills über alle Projekte hinweg
- **Plugin Skills**: Mit installierten Claude Code-Plugins gebündelt

Das SDK lädt Skills aus diesen Verzeichnissen basierend auf Ihrer `cwd`-Einstellung (aktuelles Arbeitsverzeichnis).

## Erstellen von Skills

Skills werden als Verzeichnisse definiert, die eine `SKILL.md`-Datei mit YAML-Frontmatter und Markdown-Inhalt enthalten. Das `description`-Feld bestimmt, wann Claude Ihren Skill aufruft.

**Beispiel-Verzeichnisstruktur**:
```bash
.claude/skills/processing-pdfs/
└── SKILL.md
```

Vollständige Anleitung zum Erstellen von Skills, einschließlich SKILL.md-Struktur, mehrdatei-Skills und Beispiele, finden Sie unter:
- [Agent Skills in Claude Code](https://code.claude.com/docs/skills): Vollständige Anleitung mit Beispielen
- [Agent Skills Best Practices](/docs/de/agents-and-tools/agent-skills/best-practices): Authoring-Richtlinien und Namenskonventionen

## Tool-Einschränkungen

<Note>
Das `allowed-tools`-Frontmatter-Feld in SKILL.md wird nur unterstützt, wenn Sie Claude Code CLI direkt verwenden. **Es gilt nicht, wenn Sie Skills über das SDK verwenden**.

Bei Verwendung des SDK steuern Sie den Tool-Zugriff über die Hauptoption `allowedTools` in Ihrer Abfragekonfiguration.
</Note>

Um Tools für Skills in SDK-Anwendungen einzuschränken, verwenden Sie die `allowedTools`-Option:

<Note>
Import-Anweisungen aus dem ersten Beispiel werden in den folgenden Code-Snippets angenommen.
</Note>

<CodeGroup>

```python Python
options = ClaudeAgentOptions(
    allowed_tools=["Skill", "Read", "Grep", "Glob"]  # Restricted toolset
)

async for message in query(
    prompt="Analyze the codebase structure",
    options=options
):
    print(message)
```

```typescript TypeScript
// Skills can only use Read, Grep, and Glob tools
for await (const message of query({
  prompt: "Analyze the codebase structure",
  options: {
    allowedTools: ["Skill", "Read", "Grep", "Glob"]  // Restricted toolset
  }
})) {
  console.log(message);
}
```

</CodeGroup>

## Verfügbare Skills entdecken

Um zu sehen, welche Skills in Ihrer SDK-Anwendung verfügbar sind, fragen Sie einfach Claude:

<CodeGroup>

```python Python
options = ClaudeAgentOptions(
    allowed_tools=["Skill"]
)

async for message in query(
    prompt="What Skills are available?",
    options=options
):
    print(message)
```

```typescript TypeScript
for await (const message of query({
  prompt: "What Skills are available?",
  options: {
    allowedTools: ["Skill"]
  }
})) {
  console.log(message);
}
```

</CodeGroup>

Claude listet die verfügbaren Skills basierend auf Ihrem aktuellen Arbeitsverzeichnis und installierten Plugins auf.

## Testen von Skills

Testen Sie Skills, indem Sie Fragen stellen, die ihren Beschreibungen entsprechen:

<CodeGroup>

```python Python
options = ClaudeAgentOptions(
    cwd="/path/to/project",
    allowed_tools=["Skill", "Read", "Bash"]
)

async for message in query(
    prompt="Extract text from invoice.pdf",
    options=options
):
    print(message)
```

```typescript TypeScript
for await (const message of query({
  prompt: "Extract text from invoice.pdf",
  options: {
    cwd: "/path/to/project",
    allowedTools: ["Skill", "Read", "Bash"]
  }
})) {
  console.log(message);
}
```

</CodeGroup>

Claude ruft automatisch den relevanten Skill auf, wenn die Beschreibung Ihrer Anfrage entspricht.

## Fehlerbehebung

### Skills nicht gefunden

**Arbeitsverzeichnis überprüfen**: Das SDK lädt Skills relativ zur `cwd`-Option. Stellen Sie sicher, dass sie auf ein Verzeichnis verweist, das `.claude/skills/` enthält:

<CodeGroup>

```python Python
# Ensure your cwd points to the directory containing .claude/skills/
options = ClaudeAgentOptions(
    cwd="/path/to/project",  # Must contain .claude/skills/
    allowed_tools=["Skill"]
)
```

```typescript TypeScript
// Ensure your cwd points to the directory containing .claude/skills/
const options = {
  cwd: "/path/to/project",  // Must contain .claude/skills/
  allowedTools: ["Skill"]
};
```

</CodeGroup>

Siehe den Abschnitt "Verwenden von Skills mit dem SDK" oben für das vollständige Muster.

**Dateisystem-Standort überprüfen**:
```bash
# Check project Skills
ls .claude/skills/*/SKILL.md

# Check personal Skills
ls ~/.claude/skills/*/SKILL.md
```

### Skill wird nicht verwendet

**Überprüfen Sie, ob das Skill-Tool aktiviert ist**: Bestätigen Sie, dass `"Skill"` in Ihrem `allowedTools` enthalten ist.

**Überprüfen Sie die Beschreibung**: Stellen Sie sicher, dass sie spezifisch ist und relevante Schlüsselwörter enthält. Siehe [Agent Skills Best Practices](/docs/de/agents-and-tools/agent-skills/best-practices#writing-effective-descriptions) für Anleitung zum Schreiben effektiver Beschreibungen.

### Zusätzliche Fehlerbehebung

Für allgemeine Skills-Fehlerbehebung (YAML-Syntax, Debugging usw.) siehe den [Claude Code Skills-Fehlerbehebungsabschnitt](https://code.claude.com/docs/skills#troubleshooting).

## Verwandte Dokumentation

### Skills-Leitfäden
- [Agent Skills in Claude Code](https://code.claude.com/docs/skills): Vollständiger Skills-Leitfaden mit Erstellung, Beispielen und Fehlerbehebung
- [Agent Skills-Übersicht](/docs/de/agents-and-tools/agent-skills/overview): Konzeptionelle Übersicht, Vorteile und Architektur
- [Agent Skills Best Practices](/docs/de/agents-and-tools/agent-skills/best-practices): Authoring-Richtlinien für effektive Skills
- [Agent Skills Cookbook](https://github.com/anthropics/claude-cookbooks/tree/main/skills): Beispiel-Skills und Vorlagen

### SDK-Ressourcen
- [Subagenten im SDK](/docs/de/agent-sdk/subagents): Ähnliche dateisystem-basierte Agenten mit programmatischen Optionen
- [Slash Commands im SDK](/docs/de/agent-sdk/slash-commands): Von Benutzern aufgerufene Befehle
- [SDK-Übersicht](/docs/de/agent-sdk/overview): Allgemeine SDK-Konzepte
- [TypeScript SDK-Referenz](/docs/de/agent-sdk/typescript): Vollständige API-Dokumentation
- [Python SDK-Referenz](/docs/de/agent-sdk/python): Vollständige API-Dokumentation
