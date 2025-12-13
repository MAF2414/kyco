# Subagenten im SDK

Arbeiten mit Subagenten im Claude Agent SDK

---

Subagenten im Claude Agent SDK sind spezialisierte KIs, die vom Hauptagenten orchestriert werden.
Verwenden Sie Subagenten für Kontextverwaltung und Parallelisierung.

Diese Anleitung erklärt, wie Sie Subagenten im SDK mit dem `agents`-Parameter definieren und verwenden.

## Überblick

Subagenten können bei der Verwendung des SDK auf zwei Arten definiert werden:

1. **Programmatisch** - Verwendung des `agents`-Parameters in Ihren `query()`-Optionen (empfohlen für SDK-Anwendungen)
2. **Dateisystembasiert** - Platzierung von Markdown-Dateien mit YAML-Frontmatter in bestimmten Verzeichnissen (`.claude/agents/`)

Diese Anleitung konzentriert sich hauptsächlich auf den programmatischen Ansatz mit dem `agents`-Parameter, der eine integriertere Entwicklungserfahrung für SDK-Anwendungen bietet.

## Vorteile der Verwendung von Subagenten

### Kontextverwaltung
Subagenten behalten einen separaten Kontext vom Hauptagenten bei, verhindern Informationsüberladung und halten Interaktionen fokussiert. Diese Isolation stellt sicher, dass spezialisierte Aufgaben den Hauptgesprächskontext nicht mit irrelevanten Details verschmutzen.

**Beispiel**: Ein `research-assistant`-Subagent kann Dutzende von Dateien und Dokumentationsseiten erkunden, ohne das Hauptgespräch mit all den Zwischensuchergebnissen zu überladen - er gibt nur die relevanten Erkenntnisse zurück.

### Parallelisierung
Mehrere Subagenten können gleichzeitig laufen und komplexe Arbeitsabläufe dramatisch beschleunigen.

**Beispiel**: Während einer Code-Überprüfung können Sie `style-checker`-, `security-scanner`- und `test-coverage`-Subagenten gleichzeitig ausführen und die Überprüfungszeit von Minuten auf Sekunden reduzieren.

### Spezialisierte Anweisungen und Wissen
Jeder Subagent kann maßgeschneiderte Systemprompts mit spezifischer Expertise, bewährten Praktiken und Einschränkungen haben.

**Beispiel**: Ein `database-migration`-Subagent kann detailliertes Wissen über SQL-Best-Practices, Rollback-Strategien und Datenintegritätsprüfungen haben, die unnötiges Rauschen in den Anweisungen des Hauptagenten wären.

### Werkzeugbeschränkungen
Subagenten können auf bestimmte Werkzeuge beschränkt werden, wodurch das Risiko unbeabsichtigter Aktionen reduziert wird.

**Beispiel**: Ein `doc-reviewer`-Subagent könnte nur Zugang zu Read- und Grep-Werkzeugen haben, wodurch sichergestellt wird, dass er analysieren kann, aber niemals versehentlich Ihre Dokumentationsdateien modifiziert.

## Erstellen von Subagenten

### Programmatische Definition (Empfohlen)

Definieren Sie Subagenten direkt in Ihrem Code mit dem `agents`-Parameter:

```typescript
import { query } from '@anthropic-ai/claude-agent-sdk';

const result = query({
  prompt: "Überprüfen Sie das Authentifizierungsmodul auf Sicherheitsprobleme",
  options: {
    agents: {
      'code-reviewer': {
        description: 'Experte für Code-Überprüfung. Verwenden für Qualitäts-, Sicherheits- und Wartbarkeitsüberprüfungen.',
        prompt: `Sie sind ein Code-Überprüfungsspezialist mit Expertise in Sicherheit, Leistung und bewährten Praktiken.

Bei der Code-Überprüfung:
- Identifizieren Sie Sicherheitslücken
- Prüfen Sie auf Leistungsprobleme
- Überprüfen Sie die Einhaltung von Codierungsstandards
- Schlagen Sie spezifische Verbesserungen vor

Seien Sie gründlich, aber prägnant in Ihrem Feedback.`,
        tools: ['Read', 'Grep', 'Glob'],
        model: 'sonnet'
      },
      'test-runner': {
        description: 'Führt Testsuiten aus und analysiert sie. Verwenden für Testausführung und Abdeckungsanalyse.',
        prompt: `Sie sind ein Testausführungsspezialist. Führen Sie Tests aus und liefern Sie klare Analysen der Ergebnisse.

Fokussieren Sie sich auf:
- Ausführung von Testbefehlen
- Analyse der Testausgabe
- Identifizierung fehlgeschlagener Tests
- Vorschläge für Fehlerbehebungen`,
        tools: ['Bash', 'Read', 'Grep'],
      }
    }
  }
});

for await (const message of result) {
  console.log(message);
}
```

### KYCo-Konfiguration

Wenn Sie KYCo verwenden, können Sie Subagenten in `.kyco/config.toml` unter `[agent.<agent>.agents.<name>]` definieren (z.B. `agent.claude`):

```toml
[agent.claude.agents.code-reviewer]
description = "Reviews code for bugs and style issues"
prompt = "You are a strict code reviewer."
tools = ["Read", "Grep", "Glob"]
model = "sonnet"
```

### AgentDefinition-Konfiguration

| Feld | Typ | Erforderlich | Beschreibung |
|:---|:---|:---|:---|
| `description` | `string` | Ja | Natürlichsprachige Beschreibung, wann dieser Agent verwendet werden soll |
| `prompt` | `string` | Ja | Der Systemprompt des Agenten, der seine Rolle und sein Verhalten definiert |
| `tools` | `string[]` | Nein | Array erlaubter Werkzeugnamen. Wenn weggelassen, erbt alle Werkzeuge |
| `model` | `'sonnet' \| 'opus' \| 'haiku' \| 'inherit'` | Nein | Modellüberschreibung für diesen Agenten. Standardmäßig Hauptmodell, wenn weggelassen |

### Dateisystembasierte Definition (Alternative)

Sie können Subagenten auch als Markdown-Dateien in bestimmten Verzeichnissen definieren:

- **Projektebene**: `.claude/agents/*.md` - Nur im aktuellen Projekt verfügbar
- **Benutzerebene**: `~/.claude/agents/*.md` - Projektübergreifend verfügbar

Jeder Subagent ist eine Markdown-Datei mit YAML-Frontmatter:

```markdown
---
name: code-reviewer
description: Experte für Code-Überprüfung. Verwenden für Qualitäts-, Sicherheits- und Wartbarkeitsüberprüfungen.
tools: Read, Grep, Glob, Bash
---

Der Systemprompt Ihres Subagenten steht hier. Dies definiert die Rolle,
Fähigkeiten und den Ansatz des Subagenten zur Problemlösung.
```

**Hinweis:** Programmatisch definierte Agenten (über den `agents`-Parameter) haben Vorrang vor dateisystembasierten Agenten mit demselben Namen.

## Wie das SDK Subagenten verwendet

Bei der Verwendung des Claude Agent SDK können Subagenten programmatisch definiert oder aus dem Dateisystem geladen werden. Claude wird:

1. **Programmatische Agenten laden** aus dem `agents`-Parameter in Ihren Optionen
2. **Dateisystem-Agenten automatisch erkennen** aus `.claude/agents/`-Verzeichnissen (falls nicht überschrieben)
3. **Sie automatisch aufrufen** basierend auf Aufgabenabgleich und der `description` des Agenten
4. **Ihre spezialisierten Prompts verwenden** und Werkzeugbeschränkungen
5. **Separaten Kontext beibehalten** für jeden Subagenten-Aufruf

Programmatisch definierte Agenten (über `agents`-Parameter) haben Vorrang vor dateisystembasierten Agenten mit demselben Namen.

## Beispiel-Subagenten

Für umfassende Beispiele von Subagenten einschließlich Code-Reviewern, Test-Runnern, Debuggern und Sicherheitsprüfern siehe die [Haupt-Subagenten-Anleitung](https://code.claude.com/docs/sub-agents#example-subagents). Die Anleitung enthält detaillierte Konfigurationen und bewährte Praktiken für die Erstellung effektiver Subagenten.

## SDK-Integrationsmuster

### Automatischer Aufruf

Das SDK wird automatisch geeignete Subagenten basierend auf dem Aufgabenkontext aufrufen. Stellen Sie sicher, dass das `description`-Feld Ihres Agenten klar angibt, wann er verwendet werden sollte:

```typescript
const result = query({
  prompt: "Optimieren Sie die Datenbankabfragen in der API-Schicht",
  options: {
    agents: {
      'performance-optimizer': {
        description: 'PROAKTIV verwenden, wenn Codeänderungen die Leistung beeinträchtigen könnten. MUSS für Optimierungsaufgaben verwendet werden.',
        prompt: 'Sie sind ein Leistungsoptimierungsspezialist...',
        tools: ['Read', 'Edit', 'Bash', 'Grep'],
        model: 'sonnet'
      }
    }
  }
});
```

### Expliziter Aufruf

Benutzer können bestimmte Subagenten in ihren Prompts anfordern:

```typescript
const result = query({
  prompt: "Verwenden Sie den code-reviewer-Agenten, um das Authentifizierungsmodul zu überprüfen",
  options: {
    agents: {
      'code-reviewer': {
        description: 'Experte für Code-Überprüfung',
        prompt: 'Sie sind ein sicherheitsfokussierter Code-Reviewer...',
        tools: ['Read', 'Grep', 'Glob']
      }
    }
  }
});
```

### Dynamische Agentenkonfiguration

Sie können Agenten dynamisch basierend auf den Bedürfnissen Ihrer Anwendung konfigurieren:

```typescript
import { query, type AgentDefinition } from '@anthropic-ai/claude-agent-sdk';

function createSecurityAgent(securityLevel: 'basic' | 'strict'): AgentDefinition {
  return {
    description: 'Sicherheits-Code-Reviewer',
    prompt: `Sie sind ein ${securityLevel === 'strict' ? 'strenger' : 'ausgewogener'} Sicherheitsreviewer...`,
    tools: ['Read', 'Grep', 'Glob'],
    model: securityLevel === 'strict' ? 'opus' : 'sonnet'
  };
}

const result = query({
  prompt: "Überprüfen Sie diese PR auf Sicherheitsprobleme",
  options: {
    agents: {
      'security-reviewer': createSecurityAgent('strict')
    }
  }
});
```

## Werkzeugbeschränkungen

Subagenten können über das `tools`-Feld eingeschränkten Werkzeugzugang haben:

- **Feld weglassen** - Agent erbt alle verfügbaren Werkzeuge (Standard)
- **Werkzeuge spezifizieren** - Agent kann nur aufgelistete Werkzeuge verwenden

Beispiel eines schreibgeschützten Analyseagenten:

```typescript
const result = query({
  prompt: "Analysieren Sie die Architektur dieser Codebasis",
  options: {
    agents: {
      'code-analyzer': {
        description: 'Statische Codeanalyse und Architekturüberprüfung',
        prompt: `Sie sind ein Code-Architekturanalyst. Analysieren Sie die Codestruktur,
identifizieren Sie Muster und schlagen Sie Verbesserungen vor, ohne Änderungen vorzunehmen.`,
        tools: ['Read', 'Grep', 'Glob']  // Keine Schreib- oder Ausführungsberechtigungen
      }
    }
  }
});
```

### Häufige Werkzeugkombinationen

**Schreibgeschützte Agenten** (Analyse, Überprüfung):
```typescript
tools: ['Read', 'Grep', 'Glob']
```

**Testausführungsagenten**:
```typescript
tools: ['Bash', 'Read', 'Grep']
```

**Code-Modifikationsagenten**:
```typescript
tools: ['Read', 'Edit', 'Write', 'Grep', 'Glob']
```

## Verwandte Dokumentation

- [Haupt-Subagenten-Anleitung](https://code.claude.com/docs/sub-agents) - Umfassende Subagenten-Dokumentation
- [SDK-Überblick](/docs/de/agent-sdk/overview) - Überblick über das Claude Agent SDK
- [Einstellungen](https://code.claude.com/docs/settings) - Konfigurationsdatei-Referenz
- [Slash-Befehle](https://code.claude.com/docs/slash-commands) - Erstellung benutzerdefinierter Befehle
