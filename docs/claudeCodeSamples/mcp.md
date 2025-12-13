# MCP im SDK

Erweitern Sie Claude Code mit benutzerdefinierten Tools unter Verwendung von Model Context Protocol Servern

---

## Überblick

Model Context Protocol (MCP) Server erweitern Claude Code mit benutzerdefinierten Tools und Funktionen. MCPs können als externe Prozesse ausgeführt werden, über HTTP/SSE verbinden oder direkt innerhalb Ihrer SDK-Anwendung ausgeführt werden.

## Konfiguration

### Grundkonfiguration

Konfigurieren Sie MCP-Server in `.mcp.json` im Stammverzeichnis Ihres Projekts:

<CodeGroup>

```json TypeScript
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem"],
      "env": {
        "ALLOWED_PATHS": "/Users/me/projects"
      }
    }
  }
}
```

```json Python
{
  "mcpServers": {
    "filesystem": {
      "command": "python",
      "args": ["-m", "mcp_server_filesystem"],
      "env": {
        "ALLOWED_PATHS": "/Users/me/projects"
      }
    }
  }
}
```

</CodeGroup>

### Verwendung von MCP-Servern im SDK

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Liste Dateien in meinem Projekt auf",
  options: {
    mcpConfig: ".mcp.json",
    allowedTools: ["mcp__filesystem__list_files"]
  }
})) {
  if (message.type === "result" && message.subtype === "success") {
    console.log(message.result);
  }
}
```

```python Python
from claude_agent_sdk import query

async for message in query(
    prompt="Liste Dateien in meinem Projekt auf",
    options={
        "mcpConfig": ".mcp.json",
        "allowedTools": ["mcp__filesystem__list_files"]
    }
):
    if message["type"] == "result" and message["subtype"] == "success":
        print(message["result"])
```

</CodeGroup>

## Transport-Typen

### stdio Server

Externe Prozesse, die über stdin/stdout kommunizieren:

<CodeGroup>

```typescript TypeScript
// .mcp.json Konfiguration
{
  "mcpServers": {
    "my-tool": {
      "command": "node",
      "args": ["./my-mcp-server.js"],
      "env": {
        "DEBUG": "${DEBUG:-false}"
      }
    }
  }
}
```

```python Python
# .mcp.json Konfiguration
{
  "mcpServers": {
    "my-tool": {
      "command": "python",
      "args": ["./my_mcp_server.py"],
      "env": {
        "DEBUG": "${DEBUG:-false}"
      }
    }
  }
}
```

</CodeGroup>

### HTTP/SSE Server

Remote-Server mit Netzwerkkommunikation:

<CodeGroup>

```typescript TypeScript
// SSE Server-Konfiguration
{
  "mcpServers": {
    "remote-api": {
      "type": "sse",
      "url": "https://api.example.com/mcp/sse",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}"
      }
    }
  }
}

// HTTP Server-Konfiguration
{
  "mcpServers": {
    "http-service": {
      "type": "http",
      "url": "https://api.example.com/mcp",
      "headers": {
        "X-API-Key": "${API_KEY}"
      }
    }
  }
}
```

```python Python
# SSE Server-Konfiguration
{
  "mcpServers": {
    "remote-api": {
      "type": "sse",
      "url": "https://api.example.com/mcp/sse",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}"
      }
    }
  }
}

# HTTP Server-Konfiguration
{
  "mcpServers": {
    "http-service": {
      "type": "http",
      "url": "https://api.example.com/mcp",
      "headers": {
        "X-API-Key": "${API_KEY}"
      }
    }
  }
}
```

</CodeGroup>

### SDK MCP Server

In-Process-Server, die innerhalb Ihrer Anwendung laufen. Für detaillierte Informationen zur Erstellung benutzerdefinierter Tools siehe den [Custom Tools Leitfaden](/docs/de/agent-sdk/custom-tools):

## Ressourcenverwaltung

MCP-Server können Ressourcen bereitstellen, die Claude auflisten und lesen kann:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Verfügbare Ressourcen auflisten
for await (const message of query({
  prompt: "Welche Ressourcen sind vom Datenbankserver verfügbar?",
  options: {
    mcpConfig: ".mcp.json",
    allowedTools: ["mcp__list_resources", "mcp__read_resource"]
  }
})) {
  if (message.type === "result") console.log(message.result);
}
```

```python Python
from claude_agent_sdk import query

# Verfügbare Ressourcen auflisten
async for message in query(
    prompt="Welche Ressourcen sind vom Datenbankserver verfügbar?",
    options={
        "mcpConfig": ".mcp.json",
        "allowedTools": ["mcp__list_resources", "mcp__read_resource"]
    }
):
    if message["type"] == "result":
        print(message["result"])
```

</CodeGroup>

## Authentifizierung

### Umgebungsvariablen

<CodeGroup>

```typescript TypeScript
// .mcp.json mit Umgebungsvariablen
{
  "mcpServers": {
    "secure-api": {
      "type": "sse",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}",
        "X-API-Key": "${API_KEY:-default-key}"
      }
    }
  }
}

// Umgebungsvariablen setzen
process.env.API_TOKEN = "your-token";
process.env.API_KEY = "your-key";
```

```python Python
# .mcp.json mit Umgebungsvariablen
{
  "mcpServers": {
    "secure-api": {
      "type": "sse",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}",
        "X-API-Key": "${API_KEY:-default-key}"
      }
    }
  }
}

# Umgebungsvariablen setzen
import os
os.environ["API_TOKEN"] = "your-token"
os.environ["API_KEY"] = "your-key"
```

</CodeGroup>

### OAuth2 Authentifizierung

OAuth2 MCP-Authentifizierung im Client wird derzeit nicht unterstützt.

## Fehlerbehandlung

Behandeln Sie MCP-Verbindungsfehler elegant:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Daten verarbeiten",
  options: {
    mcpServers: {
      "data-processor": dataServer
    }
  }
})) {
  if (message.type === "system" && message.subtype === "init") {
    // MCP-Server-Status überprüfen
    const failedServers = message.mcp_servers.filter(
      s => s.status !== "connected"
    );
    
    if (failedServers.length > 0) {
      console.warn("Verbindung fehlgeschlagen:", failedServers);
    }
  }
  
  if (message.type === "result" && message.subtype === "error_during_execution") {
    console.error("Ausführung fehlgeschlagen");
  }
}
```

```python Python
from claude_agent_sdk import query

async for message in query(
    prompt="Daten verarbeiten",
    options={
        "mcpServers": {
            "data-processor": data_server
        }
    }
):
    if message["type"] == "system" and message["subtype"] == "init":
        # MCP-Server-Status überprüfen
        failed_servers = [
            s for s in message["mcp_servers"]
            if s["status"] != "connected"
        ]
        
        if failed_servers:
            print(f"Verbindung fehlgeschlagen: {failed_servers}")
    
    if message["type"] == "result" and message["subtype"] == "error_during_execution":
        print("Ausführung fehlgeschlagen")
```

</CodeGroup>

## Verwandte Ressourcen

- [Custom Tools Leitfaden](/docs/de/agent-sdk/custom-tools) - Detaillierter Leitfaden zur Erstellung von SDK MCP-Servern
- [TypeScript SDK Referenz](/docs/de/agent-sdk/typescript)
- [Python SDK Referenz](/docs/de/agent-sdk/python)
- [SDK Berechtigungen](/docs/de/agent-sdk/permissions)
- [Häufige Arbeitsabläufe](https://code.claude.com/docs/common-workflows)