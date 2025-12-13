# Benutzerdefinierte Tools

Erstellen und integrieren Sie benutzerdefinierte Tools, um die Funktionalität des Claude Agent SDK zu erweitern

---

Benutzerdefinierte Tools ermöglichen es Ihnen, die Fähigkeiten von Claude Code mit Ihrer eigenen Funktionalität durch In-Process-MCP-Server zu erweitern, wodurch Claude mit externen Diensten, APIs interagieren oder spezialisierte Operationen durchführen kann.

## Erstellen benutzerdefinierter Tools

Verwenden Sie die Hilfsfunktionen `createSdkMcpServer` und `tool`, um typsichere benutzerdefinierte Tools zu definieren:

<CodeGroup>

```typescript TypeScript
import { query, tool, createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";

// Erstellen Sie einen SDK MCP-Server mit benutzerdefinierten Tools
const customServer = createSdkMcpServer({
  name: "my-custom-tools",
  version: "1.0.0",
  tools: [
    tool(
      "get_weather",
      "Aktuelle Temperatur für einen Standort anhand von Koordinaten abrufen",
      {
        latitude: z.number().describe("Breitengrad-Koordinate"),
        longitude: z.number().describe("Längengrad-Koordinate")
      },
      async (args) => {
        const response = await fetch(`https://api.open-meteo.com/v1/forecast?latitude=${args.latitude}&longitude=${args.longitude}&current=temperature_2m&temperature_unit=fahrenheit`);
        const data = await response.json();

        return {
          content: [{
            type: "text",
            text: `Temperatur: ${data.current.temperature_2m}°F`
          }]
        };
      }
    )
  ]
});
```

```python Python
from claude_agent_sdk import tool, create_sdk_mcp_server, ClaudeSDKClient, ClaudeAgentOptions
from typing import Any
import aiohttp

# Definieren Sie ein benutzerdefiniertes Tool mit dem @tool-Decorator
@tool("get_weather", "Aktuelle Temperatur für einen Standort anhand von Koordinaten abrufen", {"latitude": float, "longitude": float})
async def get_weather(args: dict[str, Any]) -> dict[str, Any]:
    # Wetter-API aufrufen
    async with aiohttp.ClientSession() as session:
        async with session.get(
            f"https://api.open-meteo.com/v1/forecast?latitude={args['latitude']}&longitude={args['longitude']}&current=temperature_2m&temperature_unit=fahrenheit"
        ) as response:
            data = await response.json()

    return {
        "content": [{
            "type": "text",
            "text": f"Temperatur: {data['current']['temperature_2m']}°F"
        }]
    }

# Erstellen Sie einen SDK MCP-Server mit dem benutzerdefinierten Tool
custom_server = create_sdk_mcp_server(
    name="my-custom-tools",
    version="1.0.0",
    tools=[get_weather]  # Übergeben Sie die dekorierte Funktion
)
```

</CodeGroup>

## Verwenden benutzerdefinierter Tools

Übergeben Sie den benutzerdefinierten Server an die `query`-Funktion über die `mcpServers`-Option als Dictionary/Objekt.

<Note>
**Wichtig:** Benutzerdefinierte MCP-Tools erfordern den Streaming-Eingabemodus. Sie müssen einen asynchronen Generator/Iterable für den `prompt`-Parameter verwenden - ein einfacher String funktioniert nicht mit MCP-Servern.
</Note>

### Tool-Namensformat

Wenn MCP-Tools Claude zur Verfügung gestellt werden, folgen ihre Namen einem spezifischen Format:
- Muster: `mcp__{server_name}__{tool_name}`
- Beispiel: Ein Tool namens `get_weather` im Server `my-custom-tools` wird zu `mcp__my-custom-tools__get_weather`

### Konfigurieren erlaubter Tools

Sie können über die `allowedTools`-Option steuern, welche Tools Claude verwenden kann:

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

// Verwenden Sie die benutzerdefinierten Tools in Ihrer Abfrage mit Streaming-Eingabe
async function* generateMessages() {
  yield {
    type: "user" as const,
    message: {
      role: "user" as const,
      content: "Wie ist das Wetter in San Francisco?"
    }
  };
}

for await (const message of query({
  prompt: generateMessages(),  // Verwenden Sie asynchronen Generator für Streaming-Eingabe
  options: {
    mcpServers: {
      "my-custom-tools": customServer  // Als Objekt/Dictionary übergeben, nicht als Array
    },
    // Optional angeben, welche Tools Claude verwenden kann
    allowedTools: [
      "mcp__my-custom-tools__get_weather",  // Das Wetter-Tool erlauben
      // Weitere Tools nach Bedarf hinzufügen
    ],
    maxTurns: 3
  }
})) {
  if (message.type === "result" && message.subtype === "success") {
    console.log(message.result);
  }
}
```

```python Python
from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions
import asyncio

# Verwenden Sie die benutzerdefinierten Tools mit Claude
options = ClaudeAgentOptions(
    mcp_servers={"my-custom-tools": custom_server},
    allowed_tools=[
        "mcp__my-custom-tools__get_weather",  # Das Wetter-Tool erlauben
        # Weitere Tools nach Bedarf hinzufügen
    ]
)

async def main():
    async with ClaudeSDKClient(options=options) as client:
        await client.query("Wie ist das Wetter in San Francisco?")

        # Antwort extrahieren und ausgeben
        async for msg in client.receive_response():
            print(msg)

asyncio.run(main())
```

</CodeGroup>

### Beispiel für mehrere Tools

Wenn Ihr MCP-Server mehrere Tools hat, können Sie sie selektiv erlauben:

<CodeGroup>

```typescript TypeScript
const multiToolServer = createSdkMcpServer({
  name: "utilities",
  version: "1.0.0",
  tools: [
    tool("calculate", "Berechnungen durchführen", { /* ... */ }, async (args) => { /* ... */ }),
    tool("translate", "Text übersetzen", { /* ... */ }, async (args) => { /* ... */ }),
    tool("search_web", "Das Web durchsuchen", { /* ... */ }, async (args) => { /* ... */ })
  ]
});

// Nur bestimmte Tools mit Streaming-Eingabe erlauben
async function* generateMessages() {
  yield {
    type: "user" as const,
    message: {
      role: "user" as const,
      content: "Berechne 5 + 3 und übersetze 'hallo' ins Spanische"
    }
  };
}

for await (const message of query({
  prompt: generateMessages(),  // Verwenden Sie asynchronen Generator für Streaming-Eingabe
  options: {
    mcpServers: {
      utilities: multiToolServer
    },
    allowedTools: [
      "mcp__utilities__calculate",   // Taschenrechner erlauben
      "mcp__utilities__translate",   // Übersetzer erlauben
      // "mcp__utilities__search_web" ist NICHT erlaubt
    ]
  }
})) {
  // Nachrichten verarbeiten
}
```

```python Python
from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions, tool, create_sdk_mcp_server
from typing import Any
import asyncio

# Definieren Sie mehrere Tools mit dem @tool-Decorator
@tool("calculate", "Berechnungen durchführen", {"expression": str})
async def calculate(args: dict[str, Any]) -> dict[str, Any]:
    result = eval(args["expression"])  # Verwenden Sie sicheres eval in der Produktion
    return {"content": [{"type": "text", "text": f"Ergebnis: {result}"}]}

@tool("translate", "Text übersetzen", {"text": str, "target_lang": str})
async def translate(args: dict[str, Any]) -> dict[str, Any]:
    # Übersetzungslogik hier
    return {"content": [{"type": "text", "text": f"Übersetzt: {args['text']}"}]}

@tool("search_web", "Das Web durchsuchen", {"query": str})
async def search_web(args: dict[str, Any]) -> dict[str, Any]:
    # Suchlogik hier
    return {"content": [{"type": "text", "text": f"Suchergebnisse für: {args['query']}"}]}

multi_tool_server = create_sdk_mcp_server(
    name="utilities",
    version="1.0.0",
    tools=[calculate, translate, search_web]  # Dekorierte Funktionen übergeben
)

# Nur bestimmte Tools mit Streaming-Eingabe erlauben
async def message_generator():
    yield {
        "type": "user",
        "message": {
            "role": "user",
            "content": "Berechne 5 + 3 und übersetze 'hallo' ins Spanische"
        }
    }

async for message in query(
    prompt=message_generator(),  # Verwenden Sie asynchronen Generator für Streaming-Eingabe
    options=ClaudeAgentOptions(
        mcp_servers={"utilities": multi_tool_server},
        allowed_tools=[
            "mcp__utilities__calculate",   # Taschenrechner erlauben
            "mcp__utilities__translate",   # Übersetzer erlauben
            # "mcp__utilities__search_web" ist NICHT erlaubt
        ]
    )
):
    if hasattr(message, 'result'):
        print(message.result)
```

</CodeGroup>

## Typsicherheit mit Python

Der `@tool`-Decorator unterstützt verschiedene Schema-Definitionsansätze für Typsicherheit:

<CodeGroup>

```typescript TypeScript
import { z } from "zod";

tool(
  "process_data",
  "Strukturierte Daten mit Typsicherheit verarbeiten",
  {
    // Zod-Schema definiert sowohl Laufzeitvalidierung als auch TypeScript-Typen
    data: z.object({
      name: z.string(),
      age: z.number().min(0).max(150),
      email: z.string().email(),
      preferences: z.array(z.string()).optional()
    }),
    format: z.enum(["json", "csv", "xml"]).default("json")
  },
  async (args) => {
    // args ist vollständig typisiert basierend auf dem Schema
    // TypeScript weiß: args.data.name ist string, args.data.age ist number, etc.
    console.log(`Verarbeite ${args.data.name}s Daten als ${args.format}`);
    
    // Ihre Verarbeitungslogik hier
    return {
      content: [{
        type: "text",
        text: `Daten für ${args.data.name} verarbeitet`
      }]
    };
  }
)
```

```python Python
from typing import Any

# Einfache Typ-Zuordnung - empfohlen für die meisten Fälle
@tool(
    "process_data",
    "Strukturierte Daten mit Typsicherheit verarbeiten",
    {
        "name": str,
        "age": int,
        "email": str,
        "preferences": list  # Optionale Parameter können in der Funktion behandelt werden
    }
)
async def process_data(args: dict[str, Any]) -> dict[str, Any]:
    # Auf Argumente mit Typ-Hinweisen für IDE-Unterstützung zugreifen
    name = args["name"]
    age = args["age"]
    email = args["email"]
    preferences = args.get("preferences", [])
    
    print(f"Verarbeite {name}s Daten (Alter: {age})")
    
    return {
        "content": [{
            "type": "text",
            "text": f"Daten für {name} verarbeitet"
        }]
    }

# Für komplexere Schemas können Sie das JSON-Schema-Format verwenden
@tool(
    "advanced_process",
    "Daten mit erweiterter Validierung verarbeiten",
    {
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer", "minimum": 0, "maximum": 150},
            "email": {"type": "string", "format": "email"},
            "format": {"type": "string", "enum": ["json", "csv", "xml"], "default": "json"}
        },
        "required": ["name", "age", "email"]
    }
)
async def advanced_process(args: dict[str, Any]) -> dict[str, Any]:
    # Verarbeitung mit erweiterter Schema-Validierung
    return {
        "content": [{
            "type": "text",
            "text": f"Erweiterte Verarbeitung für {args['name']}"
        }]
    }
```

</CodeGroup>

## Fehlerbehandlung

Behandeln Sie Fehler elegant, um aussagekräftiges Feedback zu geben:

<CodeGroup>

```typescript TypeScript
tool(
  "fetch_data",
  "Daten von einer API abrufen",
  {
    endpoint: z.string().url().describe("API-Endpunkt-URL")
  },
  async (args) => {
    try {
      const response = await fetch(args.endpoint);
      
      if (!response.ok) {
        return {
          content: [{
            type: "text",
            text: `API-Fehler: ${response.status} ${response.statusText}`
          }]
        };
      }
      
      const data = await response.json();
      return {
        content: [{
          type: "text",
          text: JSON.stringify(data, null, 2)
        }]
      };
    } catch (error) {
      return {
        content: [{
          type: "text",
          text: `Fehler beim Abrufen der Daten: ${error.message}`
        }]
      };
    }
  }
)
```

```python Python
import json
import aiohttp
from typing import Any

@tool(
    "fetch_data",
    "Daten von einer API abrufen",
    {"endpoint": str}  # Einfaches Schema
)
async def fetch_data(args: dict[str, Any]) -> dict[str, Any]:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(args["endpoint"]) as response:
                if response.status != 200:
                    return {
                        "content": [{
                            "type": "text",
                            "text": f"API-Fehler: {response.status} {response.reason}"
                        }]
                    }
                
                data = await response.json()
                return {
                    "content": [{
                        "type": "text",
                        "text": json.dumps(data, indent=2)
                    }]
                }
    except Exception as e:
        return {
            "content": [{
                "type": "text",
                "text": f"Fehler beim Abrufen der Daten: {str(e)}"
            }]
        }
```

</CodeGroup>

## Beispiel-Tools

### Datenbankabfrage-Tool

<CodeGroup>

```typescript TypeScript
const databaseServer = createSdkMcpServer({
  name: "database-tools",
  version: "1.0.0",
  tools: [
    tool(
      "query_database",
      "Eine Datenbankabfrage ausführen",
      {
        query: z.string().describe("Auszuführende SQL-Abfrage"),
        params: z.array(z.any()).optional().describe("Abfrageparameter")
      },
      async (args) => {
        const results = await db.query(args.query, args.params || []);
        return {
          content: [{
            type: "text",
            text: `${results.length} Zeilen gefunden:\n${JSON.stringify(results, null, 2)}`
          }]
        };
      }
    )
  ]
});
```

```python Python
from typing import Any
import json

@tool(
    "query_database",
    "Eine Datenbankabfrage ausführen",
    {"query": str, "params": list}  # Einfaches Schema mit list-Typ
)
async def query_database(args: dict[str, Any]) -> dict[str, Any]:
    results = await db.query(args["query"], args.get("params", []))
    return {
        "content": [{
            "type": "text",
            "text": f"{len(results)} Zeilen gefunden:\n{json.dumps(results, indent=2)}"
        }]
    }

database_server = create_sdk_mcp_server(
    name="database-tools",
    version="1.0.0",
    tools=[query_database]  # Dekorierte Funktion übergeben
)
```

</CodeGroup>

### API-Gateway-Tool

<CodeGroup>

```typescript TypeScript
const apiGatewayServer = createSdkMcpServer({
  name: "api-gateway",
  version: "1.0.0",
  tools: [
    tool(
      "api_request",
      "Authentifizierte API-Anfragen an externe Dienste stellen",
      {
        service: z.enum(["stripe", "github", "openai", "slack"]).describe("Aufzurufender Dienst"),
        endpoint: z.string().describe("API-Endpunkt-Pfad"),
        method: z.enum(["GET", "POST", "PUT", "DELETE"]).describe("HTTP-Methode"),
        body: z.record(z.any()).optional().describe("Anfragekörper"),
        query: z.record(z.string()).optional().describe("Abfrageparameter")
      },
      async (args) => {
        const config = {
          stripe: { baseUrl: "https://api.stripe.com/v1", key: process.env.STRIPE_KEY },
          github: { baseUrl: "https://api.github.com", key: process.env.GITHUB_TOKEN },
          openai: { baseUrl: "https://api.openai.com/v1", key: process.env.OPENAI_KEY },
          slack: { baseUrl: "https://slack.com/api", key: process.env.SLACK_TOKEN }
        };
        
        const { baseUrl, key } = config[args.service];
        const url = new URL(`${baseUrl}${args.endpoint}`);
        
        if (args.query) {
          Object.entries(args.query).forEach(([k, v]) => url.searchParams.set(k, v));
        }
        
        const response = await fetch(url, {
          method: args.method,
          headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
          body: args.body ? JSON.stringify(args.body) : undefined
        });
        
        const data = await response.json();
        return {
          content: [{
            type: "text",
            text: JSON.stringify(data, null, 2)
          }]
        };
      }
    )
  ]
});
```

```python Python
import os
import json
import aiohttp
from typing import Any

# Für komplexe Schemas mit Enums verwenden Sie das JSON-Schema-Format
@tool(
    "api_request",
    "Authentifizierte API-Anfragen an externe Dienste stellen",
    {
        "type": "object",
        "properties": {
            "service": {"type": "string", "enum": ["stripe", "github", "openai", "slack"]},
            "endpoint": {"type": "string"},
            "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]},
            "body": {"type": "object"},
            "query": {"type": "object"}
        },
        "required": ["service", "endpoint", "method"]
    }
)
async def api_request(args: dict[str, Any]) -> dict[str, Any]:
    config = {
        "stripe": {"base_url": "https://api.stripe.com/v1", "key": os.environ["STRIPE_KEY"]},
        "github": {"base_url": "https://api.github.com", "key": os.environ["GITHUB_TOKEN"]},
        "openai": {"base_url": "https://api.openai.com/v1", "key": os.environ["OPENAI_KEY"]},
        "slack": {"base_url": "https://slack.com/api", "key": os.environ["SLACK_TOKEN"]}
    }
    
    service_config = config[args["service"]]
    url = f"{service_config['base_url']}{args['endpoint']}"
    
    if args.get("query"):
        params = "&".join([f"{k}={v}" for k, v in args["query"].items()])
        url += f"?{params}"
    
    headers = {"Authorization": f"Bearer {service_config['key']}", "Content-Type": "application/json"}
    
    async with aiohttp.ClientSession() as session:
        async with session.request(
            args["method"], url, headers=headers, json=args.get("body")
        ) as response:
            data = await response.json()
            return {
                "content": [{
                    "type": "text",
                    "text": json.dumps(data, indent=2)
                }]
            }

api_gateway_server = create_sdk_mcp_server(
    name="api-gateway",
    version="1.0.0",
    tools=[api_request]  # Dekorierte Funktion übergeben
)
```

</CodeGroup>

### Taschenrechner-Tool

<CodeGroup>

```typescript TypeScript
const calculatorServer = createSdkMcpServer({
  name: "calculator",
  version: "1.0.0",
  tools: [
    tool(
      "calculate",
      "Mathematische Berechnungen durchführen",
      {
        expression: z.string().describe("Auszuwertender mathematischer Ausdruck"),
        precision: z.number().optional().default(2).describe("Dezimalgenauigkeit")
      },
      async (args) => {
        try {
          // Verwenden Sie eine sichere Mathematik-Evaluierungsbibliothek in der Produktion
          const result = eval(args.expression); // Nur Beispiel!
          const formatted = Number(result).toFixed(args.precision);
          
          return {
            content: [{
              type: "text",
              text: `${args.expression} = ${formatted}`
            }]
          };
        } catch (error) {
          return {
            content: [{
              type: "text",
              text: `Fehler: Ungültiger Ausdruck - ${error.message}`
            }]
          };
        }
      }
    ),
    tool(
      "compound_interest",
      "Zinseszins für eine Investition berechnen",
      {
        principal: z.number().positive().describe("Anfangsinvestitionsbetrag"),
        rate: z.number().describe("Jährlicher Zinssatz (als Dezimalzahl, z.B. 0,05 für 5%)"),
        time: z.number().positive().describe("Investitionszeitraum in Jahren"),
        n: z.number().positive().default(12).describe("Zinseszinshäufigkeit pro Jahr")
      },
      async (args) => {
        const amount = args.principal * Math.pow(1 + args.rate / args.n, args.n * args.time);
        const interest = amount - args.principal;
        
        return {
          content: [{
            type: "text",
            text: `Investitionsanalyse:\n` +
                  `Kapital: $${args.principal.toFixed(2)}\n` +
                  `Zinssatz: ${(args.rate * 100).toFixed(2)}%\n` +
                  `Zeit: ${args.time} Jahre\n` +
                  `Zinseszins: ${args.n} mal pro Jahr\n\n` +
                  `Endbetrag: $${amount.toFixed(2)}\n` +
                  `Verdiente Zinsen: $${interest.toFixed(2)}\n` +
                  `Rendite: ${((interest / args.principal) * 100).toFixed(2)}%`
          }]
        };
      }
    )
  ]
});
```

```python Python
import math
from typing import Any

@tool(
    "calculate",
    "Mathematische Berechnungen durchführen",
    {"expression": str, "precision": int}  # Einfaches Schema
)
async def calculate(args: dict[str, Any]) -> dict[str, Any]:
    try:
        # Verwenden Sie eine sichere Mathematik-Evaluierungsbibliothek in der Produktion
        result = eval(args["expression"], {"__builtins__": {}})
        precision = args.get("precision", 2)
        formatted = round(result, precision)
        
        return {
            "content": [{
                "type": "text",
                "text": f"{args['expression']} = {formatted}"
            }]
        }
    except Exception as e:
        return {
            "content": [{
                "type": "text",
                "text": f"Fehler: Ungültiger Ausdruck - {str(e)}"
            }]
        }

@tool(
    "compound_interest",
    "Zinseszins für eine Investition berechnen",
    {"principal": float, "rate": float, "time": float, "n": int}
)
async def compound_interest(args: dict[str, Any]) -> dict[str, Any]:
    principal = args["principal"]
    rate = args["rate"]
    time = args["time"]
    n = args.get("n", 12)
    
    amount = principal * (1 + rate / n) ** (n * time)
    interest = amount - principal
    
    return {
        "content": [{
            "type": "text",
            "text": f"""Investitionsanalyse:
Kapital: ${principal:.2f}
Zinssatz: {rate * 100:.2f}%
Zeit: {time} Jahre
Zinseszins: {n} mal pro Jahr

Endbetrag: ${amount:.2f}
Verdiente Zinsen: ${interest:.2f}
Rendite: {(interest / principal) * 100:.2f}%"""
        }]
    }

calculator_server = create_sdk_mcp_server(
    name="calculator",
    version="1.0.0",
    tools=[calculate, compound_interest]  # Dekorierte Funktionen übergeben
)
```

</CodeGroup>

## Verwandte Dokumentation

- [TypeScript SDK-Referenz](/docs/de/agent-sdk/typescript)
- [Python SDK-Referenz](/docs/de/agent-sdk/python)
- [MCP-Dokumentation](https://modelcontextprotocol.io)
- [SDK-Konfiguration](/docs/de/agent-sdk/overview) - Konfiguration und Einrichtung