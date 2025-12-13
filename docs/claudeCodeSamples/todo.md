# Todo-Listen

Verfolgen und anzeigen von Todos mit dem Claude Agent SDK für organisiertes Aufgabenmanagement

---

Todo-Verfolgung bietet eine strukturierte Möglichkeit, Aufgaben zu verwalten und den Fortschritt den Benutzern anzuzeigen. Das Claude Agent SDK enthält integrierte Todo-Funktionalität, die dabei hilft, komplexe Arbeitsabläufe zu organisieren und Benutzer über den Aufgabenfortschritt zu informieren.

### Todo-Lebenszyklus

Todos folgen einem vorhersagbaren Lebenszyklus:
1. **Erstellt** als `pending`, wenn Aufgaben identifiziert werden
2. **Aktiviert** zu `in_progress`, wenn die Arbeit beginnt
3. **Abgeschlossen**, wenn die Aufgabe erfolgreich beendet wird
4. **Entfernt**, wenn alle Aufgaben in einer Gruppe abgeschlossen sind

### Wann Todos verwendet werden

Das SDK erstellt automatisch Todos für:
- **Komplexe mehrstufige Aufgaben**, die 3 oder mehr unterschiedliche Aktionen erfordern
- **Vom Benutzer bereitgestellte Aufgabenlisten**, wenn mehrere Elemente erwähnt werden
- **Nicht-triviale Operationen**, die von Fortschrittsverfolgung profitieren
- **Explizite Anfragen**, wenn Benutzer um Todo-Organisation bitten

## Beispiele

### Überwachung von Todo-Änderungen

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Optimiere die Leistung meiner React-App und verfolge den Fortschritt mit Todos",
  options: { maxTurns: 15 }
})) {
  // Todo-Updates werden im Nachrichtenstrom widergespiegelt
  if (message.type === "tool_use" && message.name === "TodoWrite") {
    const todos = message.input.todos;
    
    console.log("Todo-Status-Update:");
    todos.forEach((todo, index) => {
      const status = todo.status === "completed" ? "✅" : 
                    todo.status === "in_progress" ? "🔧" : "❌";
      console.log(`${index + 1}. ${status} ${todo.content}`);
    });
  }
}
```

```python Python
from claude_agent_sdk import query

async for message in query(
    prompt="Optimiere die Leistung meiner React-App und verfolge den Fortschritt mit Todos",
    options={"max_turns": 15}
):
    # Todo-Updates werden im Nachrichtenstrom widergespiegelt
    if message.get("type") == "tool_use" and message.get("name") == "TodoWrite":
        todos = message["input"]["todos"]
        
        print("Todo-Status-Update:")
        for i, todo in enumerate(todos):
            status = "✅" if todo["status"] == "completed" else \
                    "🔧" if todo["status"] == "in_progress" else "❌"
            print(f"{i + 1}. {status} {todo['content']}")
```

</CodeGroup>

### Echtzeit-Fortschrittsanzeige

<CodeGroup>

```typescript TypeScript
import { query } from "@anthropic-ai/claude-agent-sdk";

class TodoTracker {
  private todos: any[] = [];
  
  displayProgress() {
    if (this.todos.length === 0) return;
    
    const completed = this.todos.filter(t => t.status === "completed").length;
    const inProgress = this.todos.filter(t => t.status === "in_progress").length;
    const total = this.todos.length;
    
    console.log(`\nFortschritt: ${completed}/${total} abgeschlossen`);
    console.log(`Arbeitet derzeit an: ${inProgress} Aufgabe(n)\n`);
    
    this.todos.forEach((todo, index) => {
      const icon = todo.status === "completed" ? "✅" : 
                  todo.status === "in_progress" ? "🔧" : "❌";
      const text = todo.status === "in_progress" ? todo.activeForm : todo.content;
      console.log(`${index + 1}. ${icon} ${text}`);
    });
  }
  
  async trackQuery(prompt: string) {
    for await (const message of query({
      prompt,
      options: { maxTurns: 20 }
    })) {
      if (message.type === "tool_use" && message.name === "TodoWrite") {
        this.todos = message.input.todos;
        this.displayProgress();
      }
    }
  }
}

// Verwendung
const tracker = new TodoTracker();
await tracker.trackQuery("Erstelle ein vollständiges Authentifizierungssystem mit Todos");
```

```python Python
from claude_agent_sdk import query
from typing import List, Dict

class TodoTracker:
    def __init__(self):
        self.todos: List[Dict] = []
    
    def display_progress(self):
        if not self.todos:
            return
        
        completed = len([t for t in self.todos if t["status"] == "completed"])
        in_progress = len([t for t in self.todos if t["status"] == "in_progress"])
        total = len(self.todos)
        
        print(f"\nFortschritt: {completed}/{total} abgeschlossen")
        print(f"Arbeitet derzeit an: {in_progress} Aufgabe(n)\n")
        
        for i, todo in enumerate(self.todos):
            icon = "✅" if todo["status"] == "completed" else \
                  "🔧" if todo["status"] == "in_progress" else "❌"
            text = todo["activeForm"] if todo["status"] == "in_progress" else todo["content"]
            print(f"{i + 1}. {icon} {text}")
    
    async def track_query(self, prompt: str):
        async for message in query(
            prompt=prompt,
            options={"max_turns": 20}
        ):
            if message.get("type") == "tool_use" and message.get("name") == "TodoWrite":
                self.todos = message["input"]["todos"]
                self.display_progress()

# Verwendung
tracker = TodoTracker()
await tracker.track_query("Erstelle ein vollständiges Authentifizierungssystem mit Todos")
```

</CodeGroup>

## Verwandte Dokumentation

- [TypeScript SDK Referenz](/docs/de/agent-sdk/typescript)
- [Python SDK Referenz](/docs/de/agent-sdk/python) 
- [Streaming vs Single Mode](/docs/de/agent-sdk/streaming-vs-single-mode)
- [Benutzerdefinierte Tools](/docs/de/agent-sdk/custom-tools)