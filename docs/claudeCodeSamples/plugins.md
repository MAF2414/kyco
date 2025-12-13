# Plugins im SDK

Laden Sie benutzerdefinierte Plugins, um Claude Code mit Befehlen, Agenten, Skills und Hooks über das Agent SDK zu erweitern

---

Plugins ermöglichen es Ihnen, Claude Code mit benutzerdefinierten Funktionen zu erweitern, die projektübergreifend gemeinsam genutzt werden können. Über das Agent SDK können Sie Plugins programmgesteuert aus lokalen Verzeichnissen laden, um benutzerdefinierte Schrägstrich-Befehle, Agenten, Skills, Hooks und MCP-Server zu Ihren Agent-Sitzungen hinzuzufügen.

## Was sind Plugins?

Plugins sind Pakete von Claude Code-Erweiterungen, die Folgendes enthalten können:
- **Commands**: Benutzerdefinierte Schrägstrich-Befehle
- **Agents**: Spezialisierte Subagenten für spezifische Aufgaben
- **Skills**: Von Modellen aufgerufene Funktionen, die Claude autonom nutzt
- **Hooks**: Event-Handler, die auf Tool-Nutzung und andere Ereignisse reagieren
- **MCP servers**: Externe Tool-Integrationen über Model Context Protocol

Vollständige Informationen zur Plugin-Struktur und zum Erstellen von Plugins finden Sie unter [Plugins](https://code.claude.com/docs/plugins).

## Plugins laden

Laden Sie Plugins, indem Sie ihre lokalen Dateisystempfade in Ihrer Optionskonfiguration angeben. Das SDK unterstützt das Laden mehrerer Plugins von verschiedenen Standorten.

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Hello",
  options: {
    plugins: [
      { type: "local", path: "./my-plugin" },
      { type: "local", path: "/absolute/path/to/another-plugin" }
    ]
  }
})) {
  // Plugin commands, agents, and other features are now available
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    async for message in query(
        prompt="Hello",
        options={
            "plugins": [
                {"type": "local", "path": "./my-plugin"},
                {"type": "local", "path": "/absolute/path/to/another-plugin"}
            ]
        }
    ):
        # Plugin commands, agents, and other features are now available
        pass

asyncio.run(main())
```

</CodeGroup>

## KYCO-Konfiguration (Allowlist)

KYCO kann Claude Agent SDK Plugins automatisch laden und an `options.plugins` durchreichen.
Aus Sicherheitsgründen unterstützt KYCO nur **lokale Plugins aus einer expliziten Allowlist** in der Projekt-Konfiguration.

Fügen Sie in `.kyco/config.toml` Folgendes hinzu:

```toml
[settings.claude]
allowed_plugin_paths = [
  "./plugins/my-plugin",
  "/absolute/path/to/another-plugin",
]
```

<Note>
Plugins sind Node.js-Code, der im Bridge-Prozess ausgeführt wird. Verwenden Sie nur vertrauenswürdige Plugins.
</Note>

### Pfadangaben

Plugin-Pfade können sein:
- **Relative Pfade**: Aufgelöst relativ zu Ihrem aktuellen Arbeitsverzeichnis (z. B. `"./plugins/my-plugin"`)
- **Absolute Pfade**: Vollständige Dateisystempfade (z. B. `"/home/user/plugins/my-plugin"`)

<Note>
Der Pfad sollte auf das Root-Verzeichnis des Plugins verweisen (das Verzeichnis, das `.claude-plugin/plugin.json` enthält).
</Note>

## Plugin-Installation überprüfen

Wenn Plugins erfolgreich geladen werden, erscheinen sie in der Systeminitalisierungsmeldung. Sie können überprüfen, ob Ihre Plugins verfügbar sind:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Hello",
  options: {
    plugins: [{ type: "local", path: "./my-plugin" }]
  }
})) {
  if (message.type === "system" && message.subtype === "init") {
    // Check loaded plugins
    console.log("Plugins:", message.plugins);
    // Example: [{ name: "my-plugin", path: "./my-plugin" }]

    // Check available commands from plugins
    console.log("Commands:", message.slash_commands);
    // Example: ["/help", "/compact", "my-plugin:custom-command"]
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query

async def main():
    async for message in query(
        prompt="Hello",
        options={"plugins": [{"type": "local", "path": "./my-plugin"}]}
    ):
        if message.type == "system" and message.subtype == "init":
            # Check loaded plugins
            print("Plugins:", message.data.get("plugins"))
            # Example: [{"name": "my-plugin", "path": "./my-plugin"}]

            # Check available commands from plugins
            print("Commands:", message.data.get("slash_commands"))
            # Example: ["/help", "/compact", "my-plugin:custom-command"]

asyncio.run(main())
```

</CodeGroup>

## Plugin-Befehle verwenden

Befehle von Plugins werden automatisch mit dem Plugin-Namen versehen, um Konflikte zu vermeiden. Das Format ist `plugin-name:command-name`.

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Load a plugin with a custom /greet command
for await (const message of query({
  prompt: "/my-plugin:greet",  // Use plugin command with namespace
  options: {
    plugins: [{ type: "local", path: "./my-plugin" }]
  }
})) {
  // Claude executes the custom greeting command from the plugin
  if (message.type === "assistant") {
    console.log(message.content);
  }
}
```

```python Python
import asyncio
from claude_agent_sdk import query, AssistantMessage, TextBlock

async def main():
    # Load a plugin with a custom /greet command
    async for message in query(
        prompt="/demo-plugin:greet",  # Use plugin command with namespace
        options={"plugins": [{"type": "local", "path": "./plugins/demo-plugin"}]}
    ):
        # Claude executes the custom greeting command from the plugin
        if isinstance(message, AssistantMessage):
            for block in message.content:
                if isinstance(block, TextBlock):
                    print(f"Claude: {block.text}")

asyncio.run(main())
```

</CodeGroup>

<Note>
Wenn Sie ein Plugin über die CLI installiert haben (z. B. `/plugin install my-plugin@marketplace`), können Sie es im SDK weiterhin verwenden, indem Sie seinen Installationspfad angeben. Überprüfen Sie `~/.claude/plugins/` auf CLI-installierte Plugins.
</Note>

## Vollständiges Beispiel

Hier ist ein vollständiges Beispiel, das das Laden und die Verwendung von Plugins demonstriert:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";
import * as path from "path";

async function runWithPlugin() {
  const pluginPath = path.join(__dirname, "plugins", "my-plugin");

  console.log("Loading plugin from:", pluginPath);

  for await (const message of query({
    prompt: "What custom commands do you have available?",
    options: {
      plugins: [
        { type: "local", path: pluginPath }
      ],
      maxTurns: 3
    }
  })) {
    if (message.type === "system" && message.subtype === "init") {
      console.log("Loaded plugins:", message.plugins);
      console.log("Available commands:", message.slash_commands);
    }

    if (message.type === "assistant") {
      console.log("Assistant:", message.content);
    }
  }
}

runWithPlugin().catch(console.error);
```

```python Python
#!/usr/bin/env python3
"""Example demonstrating how to use plugins with the Agent SDK."""

from pathlib import Path
import anyio
from claude_agent_sdk import (
    AssistantMessage,
    ClaudeAgentOptions,
    TextBlock,
    query,
)


async def run_with_plugin():
    """Example using a custom plugin."""
    plugin_path = Path(__file__).parent / "plugins" / "demo-plugin"

    print(f"Loading plugin from: {plugin_path}")

    options = ClaudeAgentOptions(
        plugins=[
            {"type": "local", "path": str(plugin_path)}
        ],
        max_turns=3,
    )

    async for message in query(
        prompt="What custom commands do you have available?",
        options=options
    ):
        if message.type == "system" and message.subtype == "init":
            print(f"Loaded plugins: {message.data.get('plugins')}")
            print(f"Available commands: {message.data.get('slash_commands')}")

        if isinstance(message, AssistantMessage):
            for block in message.content:
                if isinstance(block, TextBlock):
                    print(f"Assistant: {block.text}")


if __name__ == "__main__":
    anyio.run(run_with_plugin)
```

</CodeGroup>

## Plugin-Struktur-Referenz

Ein Plugin-Verzeichnis muss eine `.claude-plugin/plugin.json` Manifestdatei enthalten. Es kann optional Folgendes enthalten:

```
my-plugin/
├── .claude-plugin/
│   └── plugin.json          # Required: plugin manifest
├── commands/                 # Custom slash commands
│   └── custom-cmd.md
├── agents/                   # Custom agents
│   └── specialist.md
├── skills/                   # Agent Skills
│   └── my-skill/
│       └── SKILL.md
├── hooks/                    # Event handlers
│   └── hooks.json
└── .mcp.json                # MCP server definitions
```

Detaillierte Informationen zum Erstellen von Plugins finden Sie unter:
- [Plugins](https://code.claude.com/docs/plugins) - Vollständiger Plugin-Entwicklungsleitfaden
- [Plugins reference](https://code.claude.com/docs/plugins-reference) - Technische Spezifikationen und Schemas

## Häufige Anwendungsfälle

### Entwicklung und Tests

Laden Sie Plugins während der Entwicklung, ohne sie global zu installieren:

```typescript
plugins: [
  { type: "local", path: "./dev-plugins/my-plugin" }
]
```

### Projektspezifische Erweiterungen

Beziehen Sie Plugins in Ihr Projekt-Repository ein, um teamweite Konsistenz zu gewährleisten:

```typescript
plugins: [
  { type: "local", path: "./project-plugins/team-workflows" }
]
```

### Mehrere Plugin-Quellen

Kombinieren Sie Plugins von verschiedenen Standorten:

```typescript
plugins: [
  { type: "local", path: "./local-plugin" },
  { type: "local", path: "~/.claude/custom-plugins/shared-plugin" }
]
```

## Fehlerbehebung

### Plugin wird nicht geladen

Wenn Ihr Plugin nicht in der Init-Meldung angezeigt wird:

1. **Überprüfen Sie den Pfad**: Stellen Sie sicher, dass der Pfad auf das Plugin-Root-Verzeichnis verweist (enthält `.claude-plugin/`)
2. **Validieren Sie plugin.json**: Stellen Sie sicher, dass Ihre Manifestdatei eine gültige JSON-Syntax hat
3. **Überprüfen Sie Dateiberechtigungen**: Stellen Sie sicher, dass das Plugin-Verzeichnis lesbar ist

### Befehle nicht verfügbar

Wenn Plugin-Befehle nicht funktionieren:

1. **Verwenden Sie den Namespace**: Plugin-Befehle erfordern das Format `plugin-name:command-name`
2. **Überprüfen Sie die Init-Meldung**: Überprüfen Sie, ob der Befehl in `slash_commands` mit dem korrekten Namespace angezeigt wird
3. **Validieren Sie Befehlsdateien**: Stellen Sie sicher, dass sich Befehlsmarkdown-Dateien im Verzeichnis `commands/` befinden

### Pfadauflösungsprobleme

Wenn relative Pfade nicht funktionieren:

1. **Überprüfen Sie das Arbeitsverzeichnis**: Relative Pfade werden von Ihrem aktuellen Arbeitsverzeichnis aus aufgelöst
2. **Verwenden Sie absolute Pfade**: Verwenden Sie für Zuverlässigkeit absolute Pfade
3. **Normalisieren Sie Pfade**: Verwenden Sie Pfad-Utilities, um Pfade korrekt zu konstruieren

## Siehe auch

- [Plugins](https://code.claude.com/docs/plugins) - Vollständiger Plugin-Entwicklungsleitfaden
- [Plugins reference](https://code.claude.com/docs/plugins-reference) - Technische Spezifikationen
- [Slash Commands](/docs/de/agent-sdk/slash-commands) - Verwendung von Schrägstrich-Befehlen im SDK
- [Subagents](/docs/de/agent-sdk/subagents) - Arbeiten mit spezialisierten Agenten
- [Skills](/docs/de/agent-sdk/skills) - Verwendung von Agent Skills
