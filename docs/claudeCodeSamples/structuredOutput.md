# Strukturierte Ausgaben im SDK

Erhalten Sie validierte JSON-Ergebnisse aus Agent-Workflows

---

Erhalten Sie strukturierte, validierte JSON aus Agent-Workflows. Das Agent SDK unterstützt strukturierte Ausgaben durch JSON-Schemas und stellt sicher, dass Ihre Agenten Daten genau in dem Format zurückgeben, das Sie benötigen.

<Note>
**Wann strukturierte Ausgaben verwendet werden sollten**

Verwenden Sie strukturierte Ausgaben, wenn Sie validiertes JSON benötigen, nachdem ein Agent einen mehrstufigen Workflow mit Tools abgeschlossen hat (Dateisuchen, Befehlsausführung, Webrecherche usw.).

Für einzelne API-Aufrufe ohne Tool-Nutzung siehe [API Strukturierte Ausgaben](/docs/de/build-with-claude/structured-outputs).
</Note>

## Warum strukturierte Ausgaben verwenden

Strukturierte Ausgaben bieten zuverlässige, typsichere Integration mit Ihren Anwendungen:

- **Validierte Struktur**: Erhalten Sie immer gültiges JSON, das Ihrem Schema entspricht
- **Vereinfachte Integration**: Kein Parsing- oder Validierungscode erforderlich
- **Typsicherheit**: Verwenden Sie mit TypeScript- oder Python-Typhinweisen für End-to-End-Sicherheit
- **Saubere Trennung**: Definieren Sie Ausgabeanforderungen separat von Aufgabenanweisungen
- **Tool-Autonomie**: Agent wählt aus, welche Tools verwendet werden, während das Ausgabeformat garantiert wird

<Tabs>
<Tab title="TypeScript">

## Schnellstart

```typescript
import { query } from '@anthropic-ai/claude-agent-sdk'

const schema = {
  type: 'object',
  properties: {
    company_name: { type: 'string' },
    founded_year: { type: 'number' },
    headquarters: { type: 'string' }
  },
  required: ['company_name']
}

for await (const message of query({
  prompt: 'Research Anthropic and provide key company information',
  options: {
    outputFormat: {
      type: 'json_schema',
      schema: schema
    }
  }
})) {
  if (message.type === 'result' && message.structured_output) {
    console.log(message.structured_output)
    // { company_name: "Anthropic", founded_year: 2021, headquarters: "San Francisco, CA" }
  }
}
```

## Schemas mit Zod definieren

Für TypeScript-Projekte verwenden Sie Zod für typsichere Schemadefinition und Validierung:

```typescript
import { z } from 'zod'
import { zodToJsonSchema } from 'zod-to-json-schema'

// Schema mit Zod definieren
const AnalysisResult = z.object({
  summary: z.string(),
  issues: z.array(z.object({
    severity: z.enum(['low', 'medium', 'high']),
    description: z.string(),
    file: z.string()
  })),
  score: z.number().min(0).max(100)
})

type AnalysisResult = z.infer<typeof AnalysisResult>

// In JSON Schema konvertieren
const schema = zodToJsonSchema(AnalysisResult, { $refStrategy: 'root' })

// In Query verwenden
for await (const message of query({
  prompt: 'Analyze the codebase for security issues',
  options: {
    outputFormat: {
      type: 'json_schema',
      schema: schema
    }
  }
})) {
  if (message.type === 'result' && message.structured_output) {
    // Validieren und vollständig typiertes Ergebnis abrufen
    const parsed = AnalysisResult.safeParse(message.structured_output)
    if (parsed.success) {
      const data: AnalysisResult = parsed.data
      console.log(`Score: ${data.score}`)
      console.log(`Found ${data.issues.length} issues`)
      data.issues.forEach(issue => {
        console.log(`[${issue.severity}] ${issue.file}: ${issue.description}`)
      })
    }
  }
}
```

**Vorteile von Zod:**
- Vollständige TypeScript-Typinferenz
- Laufzeitvalidierung mit `safeParse()`
- Bessere Fehlermeldungen
- Zusammensetzbare Schemas

</Tab>
<Tab title="Python">

## Schnellstart

```python
from claude_agent_sdk import query

schema = {
    "type": "object",
    "properties": {
        "company_name": {"type": "string"},
        "founded_year": {"type": "number"},
        "headquarters": {"type": "string"}
    },
    "required": ["company_name"]
}

async for message in query(
    prompt="Research Anthropic and provide key company information",
    options={
        "output_format": {
            "type": "json_schema",
            "schema": schema
        }
    }
):
    if hasattr(message, 'structured_output'):
        print(message.structured_output)
        # {'company_name': 'Anthropic', 'founded_year': 2021, 'headquarters': 'San Francisco, CA'}
```

## Schemas mit Pydantic definieren

Für Python-Projekte verwenden Sie Pydantic für typsichere Schemadefinition und Validierung:

```python
from pydantic import BaseModel
from claude_agent_sdk import query

class Issue(BaseModel):
    severity: str  # 'low', 'medium', 'high'
    description: str
    file: str

class AnalysisResult(BaseModel):
    summary: str
    issues: list[Issue]
    score: int

# In Query verwenden
async for message in query(
    prompt="Analyze the codebase for security issues",
    options={
        "output_format": {
            "type": "json_schema",
            "schema": AnalysisResult.model_json_schema()
        }
    }
):
    if hasattr(message, 'structured_output'):
        # Validieren und vollständig typiertes Ergebnis abrufen
        result = AnalysisResult.model_validate(message.structured_output)
        print(f"Score: {result.score}")
        print(f"Found {len(result.issues)} issues")
        for issue in result.issues:
            print(f"[{issue.severity}] {issue.file}: {issue.description}")
```

**Vorteile von Pydantic:**
- Vollständige Python-Typhinweise
- Laufzeitvalidierung mit `model_validate()`
- Bessere Fehlermeldungen
- Datenklassen-Funktionalität

</Tab>
</Tabs>

## Wie strukturierte Ausgaben funktionieren

<Steps>
  <Step title="Definieren Sie Ihr JSON-Schema">
    Erstellen Sie ein JSON-Schema, das die Struktur beschreibt, die der Agent zurückgeben soll. Das Schema verwendet das Standard-JSON-Schema-Format.
  </Step>
  <Step title="Fügen Sie den outputFormat-Parameter hinzu">
    Fügen Sie den `outputFormat`-Parameter in Ihre Query-Optionen mit `type: "json_schema"` und Ihrer Schemadefinition ein.
  </Step>
  <Step title="Führen Sie Ihre Query aus">
    Der Agent verwendet alle Tools, die er benötigt, um die Aufgabe abzuschließen (Dateivorgänge, Befehle, Websuche usw.).
  </Step>
  <Step title="Greifen Sie auf validierte Ausgabe zu">
    Das Endergebnis des Agenten wird gültiges JSON sein, das Ihrem Schema entspricht und in `message.structured_output` verfügbar ist.
  </Step>
</Steps>

## Unterstützte JSON-Schema-Funktionen

Das Agent SDK unterstützt die gleichen JSON-Schema-Funktionen und Einschränkungen wie [API Strukturierte Ausgaben](/docs/de/build-with-claude/structured-outputs#json-schema-limitations).

Wichtige unterstützte Funktionen:
- Alle grundlegenden Typen: object, array, string, integer, number, boolean, null
- `enum`, `const`, `required`, `additionalProperties` (muss `false` sein)
- String-Formate: `date-time`, `date`, `email`, `uri`, `uuid` usw.
- `$ref`, `$def` und `definitions`

Für vollständige Details zu unterstützten Funktionen, Einschränkungen und Regex-Muster-Unterstützung siehe [JSON-Schema-Einschränkungen](/docs/de/build-with-claude/structured-outputs#json-schema-limitations) in der API-Dokumentation.

## Beispiel: TODO-Tracking-Agent

Hier ist ein vollständiges Beispiel, das einen Agent zeigt, der Code nach TODOs durchsucht und Git-Blame-Informationen extrahiert:

<CodeGroup>

```typescript TypeScript
import { query } from '@anthropic-ai/claude-agent-sdk'

// Struktur für TODO-Extraktion definieren
const todoSchema = {
  type: 'object',
  properties: {
    todos: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          text: { type: 'string' },
          file: { type: 'string' },
          line: { type: 'number' },
          author: { type: 'string' },
          date: { type: 'string' }
        },
        required: ['text', 'file', 'line']
      }
    },
    total_count: { type: 'number' }
  },
  required: ['todos', 'total_count']
}

// Agent verwendet Grep zum Finden von TODOs, Bash zum Abrufen von Git-Blame-Informationen
for await (const message of query({
  prompt: 'Find all TODO comments in src/ and identify who added them',
  options: {
    outputFormat: {
      type: 'json_schema',
      schema: todoSchema
    }
  }
})) {
  if (message.type === 'result' && message.structured_output) {
    const data = message.structured_output
    console.log(`Found ${data.total_count} TODOs`)
    data.todos.forEach(todo => {
      console.log(`${todo.file}:${todo.line} - ${todo.text}`)
      if (todo.author) {
        console.log(`  Added by ${todo.author} on ${todo.date}`)
      }
    })
  }
}
```

```python Python
from claude_agent_sdk import query

# Struktur für TODO-Extraktion definieren
todo_schema = {
    "type": "object",
    "properties": {
        "todos": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "file": {"type": "string"},
                    "line": {"type": "number"},
                    "author": {"type": "string"},
                    "date": {"type": "string"}
                },
                "required": ["text", "file", "line"]
            }
        },
        "total_count": {"type": "number"}
    },
    "required": ["todos", "total_count"]
}

# Agent verwendet Grep zum Finden von TODOs, Bash zum Abrufen von Git-Blame-Informationen
async for message in query(
    prompt="Find all TODO comments in src/ and identify who added them",
    options={
        "output_format": {
            "type": "json_schema",
            "schema": todo_schema
        }
    }
):
    if hasattr(message, 'structured_output'):
        data = message.structured_output
        print(f"Found {data['total_count']} TODOs")
        for todo in data['todos']:
            print(f"{todo['file']}:{todo['line']} - {todo['text']}")
            if 'author' in todo:
                print(f"  Added by {todo['author']} on {todo['date']}")
```

</CodeGroup>

Der Agent nutzt autonom die richtigen Tools (Grep, Bash), um Informationen zu sammeln und gibt validierte Daten zurück.

## Fehlerbehandlung

Wenn der Agent keine gültigen Ausgaben entsprechend Ihrem Schema erzeugen kann, erhalten Sie ein Fehlerergebnis:

```typescript
for await (const msg of query({
  prompt: 'Analyze the data',
  options: {
    outputFormat: {
      type: 'json_schema',
      schema: mySchema
    }
  }
})) {
  if (msg.type === 'result') {
    if (msg.subtype === 'success' && msg.structured_output) {
      console.log(msg.structured_output)
    } else if (msg.subtype === 'error_max_structured_output_retries') {
      console.error('Could not produce valid output')
    }
  }
}
```

## Verwandte Ressourcen

- [JSON-Schema-Dokumentation](https://json-schema.org/)
- [API Strukturierte Ausgaben](/docs/de/build-with-claude/structured-outputs) - Für einzelne API-Aufrufe
- [Benutzerdefinierte Tools](/docs/de/agent-sdk/custom-tools) - Definieren Sie Tools für Ihre Agenten
- [TypeScript SDK-Referenz](/docs/de/agent-sdk/typescript) - Vollständige TypeScript-API
- [Python SDK-Referenz](/docs/de/agent-sdk/python) - Vollständige Python-API