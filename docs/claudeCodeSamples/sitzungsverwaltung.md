# Session-Verwaltung

Verstehen, wie das Claude Agent SDK Sessions und Session-Wiederaufnahme handhabt

---

# Session-Verwaltung

Das Claude Agent SDK bietet Session-Verwaltungsfunktionen für die Handhabung von Gesprächszuständen und -wiederaufnahme. Sessions ermöglichen es Ihnen, Gespräche über mehrere Interaktionen hinweg fortzusetzen, während der vollständige Kontext beibehalten wird.

## Wie Sessions funktionieren

Wenn Sie eine neue Abfrage starten, erstellt das SDK automatisch eine Session und gibt eine Session-ID in der ersten Systemnachricht zurück. Sie können diese ID erfassen, um die Session später wieder aufzunehmen.

### Abrufen der Session-ID

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

let sessionId: string | undefined

const response = query({
  prompt: "Hilf mir beim Erstellen einer Webanwendung",
  options: {
    model: "claude-sonnet-4-5"
  }
})

for await (const message of response) {
  // Die erste Nachricht ist eine System-Init-Nachricht mit der Session-ID
  if (message.type === 'system' && message.subtype === 'init') {
    sessionId = message.session_id
    console.log(`Session gestartet mit ID: ${sessionId}`)
    // Sie können diese ID für spätere Wiederaufnahme speichern
  }

  // Andere Nachrichten verarbeiten...
  console.log(message)
}

// Später können Sie die gespeicherte sessionId verwenden, um fortzufahren
if (sessionId) {
  const resumedResponse = query({
    prompt: "Dort weitermachen, wo wir aufgehört haben",
    options: {
      resume: sessionId
    }
  })
}
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

session_id = None

async for message in query(
    prompt="Hilf mir beim Erstellen einer Webanwendung",
    options=ClaudeAgentOptions(
        model="claude-sonnet-4-5"
    )
):
    # Die erste Nachricht ist eine System-Init-Nachricht mit der Session-ID
    if hasattr(message, 'subtype') and message.subtype == 'init':
        session_id = message.data.get('session_id')
        print(f"Session gestartet mit ID: {session_id}")
        # Sie können diese ID für spätere Wiederaufnahme speichern

    # Andere Nachrichten verarbeiten...
    print(message)

# Später können Sie die gespeicherte session_id verwenden, um fortzufahren
if session_id:
    async for message in query(
        prompt="Dort weitermachen, wo wir aufgehört haben",
        options=ClaudeAgentOptions(
            resume=session_id
        )
    ):
        print(message)
```

</CodeGroup>

## Sessions wiederaufnehmen

Das SDK unterstützt die Wiederaufnahme von Sessions aus vorherigen Gesprächszuständen und ermöglicht kontinuierliche Entwicklungsworkflows. Verwenden Sie die `resume`-Option mit einer Session-ID, um ein vorheriges Gespräch fortzusetzen.

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

// Eine vorherige Session mit ihrer ID wiederaufnehmen
const response = query({
  prompt: "Setze die Implementierung des Authentifizierungssystems dort fort, wo wir aufgehört haben",
  options: {
    resume: "session-xyz", // Session-ID aus vorherigem Gespräch
    model: "claude-sonnet-4-5",
    allowedTools: ["Read", "Edit", "Write", "Glob", "Grep", "Bash"]
  }
})

// Das Gespräch wird mit vollständigem Kontext aus der vorherigen Session fortgesetzt
for await (const message of response) {
  console.log(message)
}
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

# Eine vorherige Session mit ihrer ID wiederaufnehmen
async for message in query(
    prompt="Setze die Implementierung des Authentifizierungssystems dort fort, wo wir aufgehört haben",
    options=ClaudeAgentOptions(
        resume="session-xyz",  # Session-ID aus vorherigem Gespräch
        model="claude-sonnet-4-5",
        allowed_tools=["Read", "Edit", "Write", "Glob", "Grep", "Bash"]
    )
):
    print(message)

# Das Gespräch wird mit vollständigem Kontext aus der vorherigen Session fortgesetzt
```

</CodeGroup>

Das SDK übernimmt automatisch das Laden der Gesprächshistorie und des Kontexts, wenn Sie eine Session wiederaufnehmen, wodurch Claude genau dort fortfahren kann, wo es aufgehört hat.

## Sessions verzweigen

Beim Wiederaufnehmen einer Session können Sie wählen, ob Sie die ursprüngliche Session fortsetzen oder sie in einen neuen Zweig verzweigen möchten. Standardmäßig setzt die Wiederaufnahme die ursprüngliche Session fort. Verwenden Sie die `forkSession`-Option (TypeScript) oder `fork_session`-Option (Python), um eine neue Session-ID zu erstellen, die vom wiederaufgenommenen Zustand ausgeht.

### Wann eine Session verzweigt werden sollte

Verzweigung ist nützlich, wenn Sie:
- Verschiedene Ansätze vom gleichen Ausgangspunkt aus erkunden möchten
- Mehrere Gesprächszweige erstellen möchten, ohne das Original zu verändern
- Änderungen testen möchten, ohne die ursprüngliche Session-Historie zu beeinträchtigen
- Separate Gesprächspfade für verschiedene Experimente beibehalten möchten

### Verzweigen vs. Fortsetzen

| Verhalten | `forkSession: false` (Standard) | `forkSession: true` |
|---|---|---|
| **Session-ID** | Gleich wie Original | Neue Session-ID generiert |
| **Historie** | Wird an ursprüngliche Session angehängt | Erstellt neuen Zweig vom Wiederaufnahmepunkt |
| **Ursprüngliche Session** | Verändert | Unverändert erhalten |
| **Anwendungsfall** | Lineares Gespräch fortsetzen | Verzweigen, um Alternativen zu erkunden |

### Beispiel: Eine Session verzweigen

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk"

// Zuerst die Session-ID erfassen
let sessionId: string | undefined

const response = query({
  prompt: "Hilf mir beim Entwerfen einer REST-API",
  options: { model: "claude-sonnet-4-5" }
})

for await (const message of response) {
  if (message.type === 'system' && message.subtype === 'init') {
    sessionId = message.session_id
    console.log(`Ursprüngliche Session: ${sessionId}`)
  }
}

// Die Session verzweigen, um einen anderen Ansatz zu versuchen
const forkedResponse = query({
  prompt: "Lass uns das jetzt stattdessen als GraphQL-API neu entwerfen",
  options: {
    resume: sessionId,
    forkSession: true,  // Erstellt eine neue Session-ID
    model: "claude-sonnet-4-5"
  }
})

for await (const message of forkedResponse) {
  if (message.type === 'system' && message.subtype === 'init') {
    console.log(`Verzweigte Session: ${message.session_id}`)
    // Dies wird eine andere Session-ID sein
  }
}

// Die ursprüngliche Session bleibt unverändert und kann weiterhin wiederaufgenommen werden
const originalContinued = query({
  prompt: "Füge Authentifizierung zur REST-API hinzu",
  options: {
    resume: sessionId,
    forkSession: false,  // Ursprüngliche Session fortsetzen (Standard)
    model: "claude-sonnet-4-5"
  }
})
```

```python Python
from claude_agent_sdk import query, ClaudeAgentOptions

# Zuerst die Session-ID erfassen
session_id = None

async for message in query(
    prompt="Hilf mir beim Entwerfen einer REST-API",
    options=ClaudeAgentOptions(model="claude-sonnet-4-5")
):
    if hasattr(message, 'subtype') and message.subtype == 'init':
        session_id = message.data.get('session_id')
        print(f"Ursprüngliche Session: {session_id}")

# Die Session verzweigen, um einen anderen Ansatz zu versuchen
async for message in query(
    prompt="Lass uns das jetzt stattdessen als GraphQL-API neu entwerfen",
    options=ClaudeAgentOptions(
        resume=session_id,
        fork_session=True,  # Erstellt eine neue Session-ID
        model="claude-sonnet-4-5"
    )
):
    if hasattr(message, 'subtype') and message.subtype == 'init':
        forked_id = message.data.get('session_id')
        print(f"Verzweigte Session: {forked_id}")
        # Dies wird eine andere Session-ID sein

# Die ursprüngliche Session bleibt unverändert und kann weiterhin wiederaufgenommen werden
async for message in query(
    prompt="Füge Authentifizierung zur REST-API hinzu",
    options=ClaudeAgentOptions(
        resume=session_id,
        fork_session=False,  # Ursprüngliche Session fortsetzen (Standard)
        model="claude-sonnet-4-5"
    )
):
    print(message)
```

</CodeGroup>