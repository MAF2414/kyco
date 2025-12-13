# Agent SDK Referenz - TypeScript

Vollständige API-Referenz für das TypeScript Agent SDK, einschließlich aller Funktionen, Typen und Schnittstellen.

---

<script src="/components/typescript-sdk-type-links.js" defer />

## Installation

```bash
npm install @anthropic-ai/claude-agent-sdk
```

## Funktionen

### `query()`

Die primäre Funktion für die Interaktion mit Claude Code. Erstellt einen asynchronen Generator, der Nachrichten streamt, wenn sie ankommen.

```typescript
function query({
  prompt,
  options
}: {
  prompt: string | AsyncIterable<SDKUserMessage>;
  options?: Options;
}): Query
```

#### Parameter

| Parameter | Typ | Beschreibung |
| :-------- | :--- | :---------- |
| `prompt` | `string \| AsyncIterable<`[`SDKUserMessage`](#sdkusermessage)`>` | Die Eingabeaufforderung als Zeichenkette oder asynchrones Iterable für den Streaming-Modus |
| `options` | [`Options`](#options) | Optionales Konfigurationsobjekt (siehe Options-Typ unten) |

#### Rückgabewert

Gibt ein [`Query`](#query-1)-Objekt zurück, das `AsyncGenerator<`[`SDKMessage`](#sdkmessage)`, void>` mit zusätzlichen Methoden erweitert.

### `tool()`

Erstellt eine typsichere MCP-Tool-Definition zur Verwendung mit SDK MCP-Servern.

```typescript
function tool<Schema extends ZodRawShape>(
  name: string,
  description: string,
  inputSchema: Schema,
  handler: (args: z.infer<ZodObject<Schema>>, extra: unknown) => Promise<CallToolResult>
): SdkMcpToolDefinition<Schema>
```

#### Parameter

| Parameter | Typ | Beschreibung |
| :-------- | :--- | :---------- |
| `name` | `string` | Der Name des Tools |
| `description` | `string` | Eine Beschreibung, was das Tool tut |
| `inputSchema` | `Schema extends ZodRawShape` | Zod-Schema, das die Eingabeparameter des Tools definiert |
| `handler` | `(args, extra) => Promise<`[`CallToolResult`](#calltoolresult)`>` | Asynchrone Funktion, die die Tool-Logik ausführt |

### `createSdkMcpServer()`

Erstellt eine MCP-Server-Instanz, die im gleichen Prozess wie Ihre Anwendung ausgeführt wird.

```typescript
function createSdkMcpServer(options: {
  name: string;
  version?: string;
  tools?: Array<SdkMcpToolDefinition<any>>;
}): McpSdkServerConfigWithInstance
```

#### Parameter

| Parameter | Typ | Beschreibung |
| :-------- | :--- | :---------- |
| `options.name` | `string` | Der Name des MCP-Servers |
| `options.version` | `string` | Optionale Versionszeichenkette |
| `options.tools` | `Array<SdkMcpToolDefinition>` | Array von Tool-Definitionen, die mit [`tool()`](#tool) erstellt wurden |

## Typen

### `Options`

Konfigurationsobjekt für die `query()`-Funktion.

| Eigenschaft | Typ | Standard | Beschreibung |
| :------- | :--- | :------ | :---------- |
| `abortController` | `AbortController` | `new AbortController()` | Controller zum Abbrechen von Operationen |
| `additionalDirectories` | `string[]` | `[]` | Zusätzliche Verzeichnisse, auf die Claude zugreifen kann |
| `agents` | `Record<string, [`AgentDefinition`](#agentdefinition)>` | `undefined` | Programmatische Definition von Subagenten |
| `allowedTools` | `string[]` | Alle Tools | Liste der zulässigen Tool-Namen |
| `canUseTool` | [`CanUseTool`](#canusetool) | `undefined` | Benutzerdefinierte Berechtigungsfunktion für die Tool-Nutzung |
| `continue` | `boolean` | `false` | Setzen Sie die neueste Konversation fort |
| `cwd` | `string` | `process.cwd()` | Aktuelles Arbeitsverzeichnis |
| `disallowedTools` | `string[]` | `[]` | Liste der nicht zulässigen Tool-Namen |
| `env` | `Dict<string>` | `process.env` | Umgebungsvariablen |
| `executable` | `'bun' \| 'deno' \| 'node'` | Automatisch erkannt | Zu verwendende JavaScript-Laufzeit |
| `executableArgs` | `string[]` | `[]` | Argumente, die an die ausführbare Datei übergeben werden |
| `extraArgs` | `Record<string, string \| null>` | `{}` | Zusätzliche Argumente |
| `fallbackModel` | `string` | `undefined` | Modell, das verwendet wird, wenn das primäre fehlschlägt |
| `forkSession` | `boolean` | `false` | Beim Fortsetzen mit `resume` zu einer neuen Sitzungs-ID verzweigen, anstatt die ursprüngliche Sitzung fortzusetzen |
| `hooks` | `Partial<Record<`[`HookEvent`](#hookevent)`, `[`HookCallbackMatcher`](#hookcallbackmatcher)`[]>>` | `{}` | Hook-Callbacks für Ereignisse |
| `includePartialMessages` | `boolean` | `false` | Teilweise Nachrichtenereignisse einschließen |
| `maxThinkingTokens` | `number` | `undefined` | Maximale Token für den Denkprozess |
| `maxTurns` | `number` | `undefined` | Maximale Konversationsrunden |
| `mcpServers` | `Record<string, [`McpServerConfig`](#mcpserverconfig)>` | `{}` | MCP-Server-Konfigurationen |
| `model` | `string` | Standard aus CLI | Zu verwendendes Claude-Modell |
| `outputFormat` | `{ type: 'json_schema', schema: JSONSchema }` | `undefined` | Definieren Sie das Ausgabeformat für Agent-Ergebnisse. Siehe [Strukturierte Ausgaben](/docs/de/agent-sdk/structured-outputs) für Details |
| `pathToClaudeCodeExecutable` | `string` | Verwendet integrierte ausführbare Datei | Pfad zur Claude Code-ausführbaren Datei |
| `permissionMode` | [`PermissionMode`](#permissionmode) | `'default'` | Berechtigungsmodus für die Sitzung |
| `permissionPromptToolName` | `string` | `undefined` | MCP-Tool-Name für Berechtigungsaufforderungen |
| `plugins` | [`SdkPluginConfig`](#sdkpluginconfig)`[]` | `[]` | Laden Sie benutzerdefinierte Plugins aus lokalen Pfaden. Siehe [Plugins](/docs/de/agent-sdk/plugins) für Details |
| `resume` | `string` | `undefined` | Sitzungs-ID zum Fortsetzen |
| `settingSources` | [`SettingSource`](#settingsource)`[]` | `[]` (keine Einstellungen) | Steuern Sie, welche Dateisystem-Einstellungen geladen werden. Wenn weggelassen, werden keine Einstellungen geladen. **Hinweis:** Muss `'project'` enthalten, um CLAUDE.md-Dateien zu laden |
| `stderr` | `(data: string) => void` | `undefined` | Callback für stderr-Ausgabe |
| `strictMcpConfig` | `boolean` | `false` | Erzwingen Sie strikte MCP-Validierung |
| `systemPrompt` | `string \| { type: 'preset'; preset: 'claude_code'; append?: string }` | `undefined` (leere Aufforderung) | Systemanforderungskonfiguration. Übergeben Sie eine Zeichenkette für eine benutzerdefinierte Aufforderung oder `{ type: 'preset', preset: 'claude_code' }`, um die Systemaufforderung von Claude Code zu verwenden. Wenn Sie die Preset-Objektform verwenden, fügen Sie `append` hinzu, um die Systemaufforderung mit zusätzlichen Anweisungen zu erweitern |

### `Query`

Schnittstelle, die von der `query()`-Funktion zurückgegeben wird.

```typescript
interface Query extends AsyncGenerator<SDKMessage, void> {
  interrupt(): Promise<void>;
  setPermissionMode(mode: PermissionMode): Promise<void>;
}
```

#### Methoden

| Methode | Beschreibung |
| :----- | :---------- |
| `interrupt()` | Unterbricht die Abfrage (nur im Streaming-Eingabemodus verfügbar) |
| `setPermissionMode()` | Ändert den Berechtigungsmodus (nur im Streaming-Eingabemodus verfügbar) |

### `AgentDefinition`

Konfiguration für einen programmatisch definierten Subagenten.

```typescript
type AgentDefinition = {
  description: string;
  tools?: string[];
  prompt: string;
  model?: 'sonnet' | 'opus' | 'haiku' | 'inherit';
}
```

| Feld | Erforderlich | Beschreibung |
|:------|:---------|:------------|
| `description` | Ja | Natürlichsprachige Beschreibung, wann dieser Agent verwendet werden soll |
| `tools` | Nein | Array von zulässigen Tool-Namen. Wenn weggelassen, erbt alle Tools |
| `prompt` | Ja | Die Systemaufforderung des Agenten |
| `model` | Nein | Modellüberschreibung für diesen Agenten. Wenn weggelassen, verwendet das Hauptmodell |

### `SettingSource`

Steuert, welche dateisystembasierte Konfigurationsquellen das SDK Einstellungen aus lädt.

```typescript
type SettingSource = 'user' | 'project' | 'local';
```

| Wert | Beschreibung | Ort |
|:------|:------------|:---------|
| `'user'` | Globale Benutzereinstellungen | `~/.claude/settings.json` |
| `'project'` | Gemeinsame Projekteinstellungen (versionskontrolliert) | `.claude/settings.json` |
| `'local'` | Lokale Projekteinstellungen (gitignoriert) | `.claude/settings.local.json` |

#### Standardverhalten

Wenn `settingSources` **weggelassen** oder **undefined** ist, lädt das SDK **keine** Dateisystem-Einstellungen. Dies bietet Isolation für SDK-Anwendungen.

#### Warum settingSources verwenden?

**Laden Sie alle Dateisystem-Einstellungen (Legacy-Verhalten):**
```typescript
// Laden Sie alle Einstellungen wie SDK v0.0.x
const result = query({
  prompt: "Analyze this code",
  options: {
    settingSources: ['user', 'project', 'local']  // Laden Sie alle Einstellungen
  }
});
```

**Laden Sie nur bestimmte Einstellungsquellen:**
```typescript
// Laden Sie nur Projekteinstellungen, ignorieren Sie Benutzer und lokal
const result = query({
  prompt: "Run CI checks",
  options: {
    settingSources: ['project']  // Nur .claude/settings.json
  }
});
```

**Test- und CI-Umgebungen:**
```typescript
// Stellen Sie konsistentes Verhalten in CI sicher, indem Sie lokale Einstellungen ausschließen
const result = query({
  prompt: "Run tests",
  options: {
    settingSources: ['project'],  // Nur teamweit gemeinsame Einstellungen
    permissionMode: 'bypassPermissions'
  }
});
```

**SDK-only-Anwendungen:**
```typescript
// Definieren Sie alles programmatisch (Standardverhalten)
// Keine Dateisystem-Abhängigkeiten - settingSources standardmäßig auf []
const result = query({
  prompt: "Review this PR",
  options: {
    // settingSources: [] ist die Standardeinstellung, keine Angabe erforderlich
    agents: { /* ... */ },
    mcpServers: { /* ... */ },
    allowedTools: ['Read', 'Grep', 'Glob']
  }
});
```

**Laden von CLAUDE.md-Projektanweisungen:**
```typescript
// Laden Sie Projekteinstellungen, um CLAUDE.md-Dateien einzuschließen
const result = query({
  prompt: "Add a new feature following project conventions",
  options: {
    systemPrompt: {
      type: 'preset',
      preset: 'claude_code'  // Erforderlich, um CLAUDE.md zu verwenden
    },
    settingSources: ['project'],  // Lädt CLAUDE.md aus dem Projektverzeichnis
    allowedTools: ['Read', 'Write', 'Edit']
  }
});
```

#### Einstellungspriorität

Wenn mehrere Quellen geladen werden, werden Einstellungen mit dieser Priorität zusammengeführt (höchste zu niedrigste):
1. Lokale Einstellungen (`.claude/settings.local.json`)
2. Projekteinstellungen (`.claude/settings.json`)
3. Benutzereinstellungen (`~/.claude/settings.json`)

Programmatische Optionen (wie `agents`, `allowedTools`) überschreiben immer Dateisystem-Einstellungen.

### `PermissionMode`

```typescript
type PermissionMode =
  | 'default'           // Standardberechtigungsverhalten
  | 'acceptEdits'       // Automatische Akzeptanz von Dateibearbeitungen
  | 'bypassPermissions' // Umgehen Sie alle Berechtigungsprüfungen
  | 'plan'              // Planungsmodus - keine Ausführung
```

### `CanUseTool`

Benutzerdefinierter Berechtigungsfunktionstyp zur Steuerung der Tool-Nutzung.

```typescript
type CanUseTool = (
  toolName: string,
  input: ToolInput,
  options: {
    signal: AbortSignal;
    suggestions?: PermissionUpdate[];
  }
) => Promise<PermissionResult>;
```

### `PermissionResult`

Ergebnis einer Berechtigungsprüfung.

```typescript
type PermissionResult = 
  | {
      behavior: 'allow';
      updatedInput: ToolInput;
      updatedPermissions?: PermissionUpdate[];
    }
  | {
      behavior: 'deny';
      message: string;
      interrupt?: boolean;
    }
```

### `McpServerConfig`

Konfiguration für MCP-Server.

```typescript
type McpServerConfig = 
  | McpStdioServerConfig
  | McpSSEServerConfig
  | McpHttpServerConfig
  | McpSdkServerConfigWithInstance;
```

#### `McpStdioServerConfig`

```typescript
type McpStdioServerConfig = {
  type?: 'stdio';
  command: string;
  args?: string[];
  env?: Record<string, string>;
}
```

#### `McpSSEServerConfig`

```typescript
type McpSSEServerConfig = {
  type: 'sse';
  url: string;
  headers?: Record<string, string>;
}
```

#### `McpHttpServerConfig`

```typescript
type McpHttpServerConfig = {
  type: 'http';
  url: string;
  headers?: Record<string, string>;
}
```

#### `McpSdkServerConfigWithInstance`

```typescript
type McpSdkServerConfigWithInstance = {
  type: 'sdk';
  name: string;
  instance: McpServer;
}
```

### `SdkPluginConfig`

Konfiguration zum Laden von Plugins im SDK.

```typescript
type SdkPluginConfig = {
  type: 'local';
  path: string;
}
```

| Feld | Typ | Beschreibung |
|:------|:-----|:------------|
| `type` | `'local'` | Muss `'local'` sein (derzeit nur lokale Plugins unterstützt) |
| `path` | `string` | Absoluter oder relativer Pfad zum Plugin-Verzeichnis |

**Beispiel:**
```typescript
plugins: [
  { type: 'local', path: './my-plugin' },
  { type: 'local', path: '/absolute/path/to/plugin' }
]
```

Vollständige Informationen zum Erstellen und Verwenden von Plugins finden Sie unter [Plugins](/docs/de/agent-sdk/plugins).

## Nachrichtentypen

### `SDKMessage`

Union-Typ aller möglichen Nachrichten, die von der Abfrage zurückgegeben werden.

```typescript
type SDKMessage = 
  | SDKAssistantMessage
  | SDKUserMessage
  | SDKUserMessageReplay
  | SDKResultMessage
  | SDKSystemMessage
  | SDKPartialAssistantMessage
  | SDKCompactBoundaryMessage;
```

### `SDKAssistantMessage`

Antwortnachricht des Assistenten.

```typescript
type SDKAssistantMessage = {
  type: 'assistant';
  uuid: UUID;
  session_id: string;
  message: APIAssistantMessage; // Aus Anthropic SDK
  parent_tool_use_id: string | null;
}
```

### `SDKUserMessage`

Benutzereingabenachricht.

```typescript
type SDKUserMessage = {
  type: 'user';
  uuid?: UUID;
  session_id: string;
  message: APIUserMessage; // Aus Anthropic SDK
  parent_tool_use_id: string | null;
}
```

### `SDKUserMessageReplay`

Wiedergegebene Benutzernachricht mit erforderlicher UUID.

```typescript
type SDKUserMessageReplay = {
  type: 'user';
  uuid: UUID;
  session_id: string;
  message: APIUserMessage;
  parent_tool_use_id: string | null;
}
```

### `SDKResultMessage`

Endgültige Ergebnisnachricht.

```typescript
type SDKResultMessage = 
  | {
      type: 'result';
      subtype: 'success';
      uuid: UUID;
      session_id: string;
      duration_ms: number;
      duration_api_ms: number;
      is_error: boolean;
      num_turns: number;
      result: string;
      total_cost_usd: number;
      usage: NonNullableUsage;
      permission_denials: SDKPermissionDenial[];
    }
  | {
      type: 'result';
      subtype: 'error_max_turns' | 'error_during_execution';
      uuid: UUID;
      session_id: string;
      duration_ms: number;
      duration_api_ms: number;
      is_error: boolean;
      num_turns: number;
      total_cost_usd: number;
      usage: NonNullableUsage;
      permission_denials: SDKPermissionDenial[];
    }
```

### `SDKSystemMessage`

Systeminitalisierungsnachricht.

```typescript
type SDKSystemMessage = {
  type: 'system';
  subtype: 'init';
  uuid: UUID;
  session_id: string;
  apiKeySource: ApiKeySource;
  cwd: string;
  tools: string[];
  mcp_servers: {
    name: string;
    status: string;
  }[];
  model: string;
  permissionMode: PermissionMode;
  slash_commands: string[];
  output_style: string;
}
```

### `SDKPartialAssistantMessage`

Streaming-Teilnachricht (nur wenn `includePartialMessages` wahr ist).

```typescript
type SDKPartialAssistantMessage = {
  type: 'stream_event';
  event: RawMessageStreamEvent; // Aus Anthropic SDK
  parent_tool_use_id: string | null;
  uuid: UUID;
  session_id: string;
}
```

### `SDKCompactBoundaryMessage`

Nachricht, die eine Konversationskomprimierungsgrenze anzeigt.

```typescript
type SDKCompactBoundaryMessage = {
  type: 'system';
  subtype: 'compact_boundary';
  uuid: UUID;
  session_id: string;
  compact_metadata: {
    trigger: 'manual' | 'auto';
    pre_tokens: number;
  };
}
```

### `SDKPermissionDenial`

Informationen über eine verweigerte Tool-Nutzung.

```typescript
type SDKPermissionDenial = {
  tool_name: string;
  tool_use_id: string;
  tool_input: ToolInput;
}
```

## Hook-Typen

### `HookEvent`

Verfügbare Hook-Ereignisse.

```typescript
type HookEvent = 
  | 'PreToolUse'
  | 'PostToolUse'
  | 'PostToolUseFailure'
  | 'Notification'
  | 'UserPromptSubmit'
  | 'SessionStart'
  | 'SessionEnd'
  | 'Stop'
  | 'SubagentStart'
  | 'SubagentStop'
  | 'PreCompact'
  | 'PermissionRequest';
```

### `HookCallback`

Hook-Callback-Funktionstyp.

```typescript
type HookCallback = (
  input: HookInput, // Union aller Hook-Eingabetypen
  toolUseID: string | undefined,
  options: { signal: AbortSignal }
) => Promise<HookJSONOutput>;
```

### `HookCallbackMatcher`

Hook-Konfiguration mit optionalem Matcher.

```typescript
interface HookCallbackMatcher {
  matcher?: string;
  hooks: HookCallback[];
  /** Timeout in seconds for all hooks in this matcher */
  timeout?: number;
}
```

### `HookInput`

Union-Typ aller Hook-Eingabetypen.

```typescript
type HookInput = 
  | PreToolUseHookInput
  | PostToolUseHookInput
  | PostToolUseFailureHookInput
  | NotificationHookInput
  | UserPromptSubmitHookInput
  | SessionStartHookInput
  | SessionEndHookInput
  | StopHookInput
  | SubagentStartHookInput
  | SubagentStopHookInput
  | PreCompactHookInput
  | PermissionRequestHookInput;
```

### `BaseHookInput`

Basisschnittstelle, die alle Hook-Eingabetypen erweitern.

```typescript
type BaseHookInput = {
  session_id: string;
  transcript_path: string;
  cwd: string;
  permission_mode?: string;
}
```

#### `PreToolUseHookInput`

```typescript
type PreToolUseHookInput = BaseHookInput & {
  hook_event_name: 'PreToolUse';
  tool_name: string;
  tool_input: ToolInput;
  tool_use_id: string;
}
```

#### `PostToolUseHookInput`

```typescript
type PostToolUseHookInput = BaseHookInput & {
  hook_event_name: 'PostToolUse';
  tool_name: string;
  tool_input: ToolInput;
  tool_response: ToolOutput;
  tool_use_id: string;
}
```

#### `PostToolUseFailureHookInput`

```typescript
type PostToolUseFailureHookInput = BaseHookInput & {
  hook_event_name: 'PostToolUseFailure';
  tool_name: string;
  tool_input: ToolInput;
  tool_use_id: string;
  error: string;
  is_interrupt?: boolean;
}
```

#### `NotificationHookInput`

```typescript
type NotificationHookInput = BaseHookInput & {
  hook_event_name: 'Notification';
  message: string;
  title?: string;
  notification_type: string;
}
```

#### `PermissionRequestHookInput`

```typescript
type PermissionRequestHookInput = BaseHookInput & {
  hook_event_name: 'PermissionRequest';
  tool_name: string;
  tool_input: ToolInput;
  permission_suggestions?: PermissionUpdate[];
}
```

#### `UserPromptSubmitHookInput`

```typescript
type UserPromptSubmitHookInput = BaseHookInput & {
  hook_event_name: 'UserPromptSubmit';
  prompt: string;
}
```

#### `SessionStartHookInput`

```typescript
type SessionStartHookInput = BaseHookInput & {
  hook_event_name: 'SessionStart';
  source: 'startup' | 'resume' | 'clear' | 'compact';
}
```

#### `SessionEndHookInput`

```typescript
type SessionEndHookInput = BaseHookInput & {
  hook_event_name: 'SessionEnd';
  reason: 'clear' | 'logout' | 'prompt_input_exit' | 'other';
}
```

#### `StopHookInput`

```typescript
type StopHookInput = BaseHookInput & {
  hook_event_name: 'Stop';
  stop_hook_active: boolean;
}
```

#### `SubagentStartHookInput`

```typescript
type SubagentStartHookInput = BaseHookInput & {
  hook_event_name: 'SubagentStart';
  agent_id: string;
  agent_type: string;
}
```

#### `SubagentStopHookInput`

```typescript
type SubagentStopHookInput = BaseHookInput & {
  hook_event_name: 'SubagentStop';
  stop_hook_active: boolean;
  agent_id: string;
  agent_transcript_path: string;
}
```

#### `PreCompactHookInput`

```typescript
type PreCompactHookInput = BaseHookInput & {
  hook_event_name: 'PreCompact';
  trigger: 'manual' | 'auto';
  custom_instructions: string | null;
}
```

### `HookJSONOutput`

Hook-Rückgabewert.

```typescript
type HookJSONOutput = AsyncHookJSONOutput | SyncHookJSONOutput;
```

#### `AsyncHookJSONOutput`

```typescript
type AsyncHookJSONOutput = {
  async: true;
  asyncTimeout?: number;
}
```

#### `SyncHookJSONOutput`

```typescript
type SyncHookJSONOutput = {
  continue?: boolean;
  suppressOutput?: boolean;
  stopReason?: string;
  decision?: 'approve' | 'block';
  systemMessage?: string;
  reason?: string;
  hookSpecificOutput?:
    | {
        hookEventName: 'PreToolUse';
        permissionDecision?: 'allow' | 'deny' | 'ask';
        permissionDecisionReason?: string;
      }
    | {
        hookEventName: 'UserPromptSubmit';
        additionalContext?: string;
      }
    | {
        hookEventName: 'SessionStart';
        additionalContext?: string;
      }
    | {
        hookEventName: 'PostToolUse';
        additionalContext?: string;
      };
}
```

### KYCO Bridge (Hooks)

Die KYCO SDK Bridge kann ausgewählte Hook-Events zusätzlich als NDJSON streamen.

- Aktivierung: `hooks.events: ['PreToolUse']` im Request an `POST /claude/query`
- Event (Phase 1): `hook.pre_tool_use` vor jedem Tool-Call

Beispiel (Request):

```json
{
  "prompt": "…",
  "cwd": "/path/to/repo",
  "hooks": { "events": ["PreToolUse"] }
}
```

Beispiel (NDJSON Event):

```json
{"type":"hook.pre_tool_use","sessionId":"...","timestamp":0,"toolName":"Bash","toolInput":{},"toolUseId":"...","transcriptPath":"..."}
```

## Tool-Eingabetypen

Dokumentation von Eingabeschemas für alle integrierten Claude Code-Tools. Diese Typen werden aus `@anthropic-ai/claude-agent-sdk` exportiert und können für typsichere Tool-Interaktionen verwendet werden.

### `ToolInput`

**Hinweis:** Dies ist ein reiner Dokumentationstyp zur Verdeutlichung. Er stellt die Union aller Tool-Eingabetypen dar.

```typescript
type ToolInput = 
  | AgentInput
  | BashInput
  | BashOutputInput
  | FileEditInput
  | FileReadInput
  | FileWriteInput
  | GlobInput
  | GrepInput
  | KillShellInput
  | NotebookEditInput
  | WebFetchInput
  | WebSearchInput
  | TodoWriteInput
  | ExitPlanModeInput
  | ListMcpResourcesInput
  | ReadMcpResourceInput;
```

### Task

**Tool-Name:** `Task`

```typescript
interface AgentInput {
  /**
   * Eine kurze (3-5 Wörter) Beschreibung der Aufgabe
   */
  description: string;
  /**
   * Die Aufgabe, die der Agent ausführen soll
   */
  prompt: string;
  /**
   * Der Typ des spezialisierten Agenten, der für diese Aufgabe verwendet werden soll
   */
  subagent_type: string;
}
```

Startet einen neuen Agenten, um komplexe, mehrstufige Aufgaben autonom zu bewältigen.

### Bash

**Tool-Name:** `Bash`

```typescript
interface BashInput {
  /**
   * Der auszuführende Befehl
   */
  command: string;
  /**
   * Optionales Timeout in Millisekunden (max 600000)
   */
  timeout?: number;
  /**
   * Klare, prägnante Beschreibung, was dieser Befehl in 5-10 Wörtern tut
   */
  description?: string;
  /**
   * Setzen Sie auf true, um diesen Befehl im Hintergrund auszuführen
   */
  run_in_background?: boolean;
}
```

Führt Bash-Befehle in einer persistenten Shell-Sitzung mit optionalem Timeout und Hintergrundausführung aus.

### BashOutput

**Tool-Name:** `BashOutput`

```typescript
interface BashOutputInput {
  /**
   * Die ID der Hintergrund-Shell, von der die Ausgabe abgerufen werden soll
   */
  bash_id: string;
  /**
   * Optionaler Regex zum Filtern von Ausgabezeilen
   */
  filter?: string;
}
```

Ruft die Ausgabe von einer laufenden oder abgeschlossenen Hintergrund-Bash-Shell ab.

### Edit

**Tool-Name:** `Edit`

```typescript
interface FileEditInput {
  /**
   * Der absolute Pfad zur zu ändernden Datei
   */
  file_path: string;
  /**
   * Der zu ersetzende Text
   */
  old_string: string;
  /**
   * Der Text, durch den er ersetzt werden soll (muss sich von old_string unterscheiden)
   */
  new_string: string;
  /**
   * Ersetzen Sie alle Vorkommen von old_string (Standard false)
   */
  replace_all?: boolean;
}
```

Führt exakte Zeichenkettenersetzungen in Dateien durch.

### Read

**Tool-Name:** `Read`

```typescript
interface FileReadInput {
  /**
   * Der absolute Pfad zur zu lesenden Datei
   */
  file_path: string;
  /**
   * Die Zeilennummer, ab der gelesen werden soll
   */
  offset?: number;
  /**
   * Die Anzahl der zu lesenden Zeilen
   */
  limit?: number;
}
```

Liest Dateien aus dem lokalen Dateisystem, einschließlich Text, Bilder, PDFs und Jupyter-Notebooks.

### Write

**Tool-Name:** `Write`

```typescript
interface FileWriteInput {
  /**
   * Der absolute Pfad zur zu schreibenden Datei
   */
  file_path: string;
  /**
   * Der in die Datei zu schreibende Inhalt
   */
  content: string;
}
```

Schreibt eine Datei in das lokale Dateisystem und überschreibt sie, falls vorhanden.

### Glob

**Tool-Name:** `Glob`

```typescript
interface GlobInput {
  /**
   * Das Glob-Muster, das mit Dateien abgeglichen werden soll
   */
  pattern: string;
  /**
   * Das Verzeichnis, in dem gesucht werden soll (Standard cwd)
   */
  path?: string;
}
```

Schnelle Dateimusterabstimmung, die mit jeder Codebasis-Größe funktioniert.

### Grep

**Tool-Name:** `Grep`

```typescript
interface GrepInput {
  /**
   * Das reguläre Ausdrucksmuster, nach dem gesucht werden soll
   */
  pattern: string;
  /**
   * Datei oder Verzeichnis, in dem gesucht werden soll (Standard cwd)
   */
  path?: string;
  /**
   * Glob-Muster zum Filtern von Dateien (z. B. "*.js")
   */
  glob?: string;
  /**
   * Dateityp, in dem gesucht werden soll (z. B. "js", "py", "rust")
   */
  type?: string;
  /**
   * Ausgabemodus: "content", "files_with_matches" oder "count"
   */
  output_mode?: 'content' | 'files_with_matches' | 'count';
  /**
   * Suche ohne Berücksichtigung der Groß-/Kleinschreibung
   */
  '-i'?: boolean;
  /**
   * Zeilennummern anzeigen (für Content-Modus)
   */
  '-n'?: boolean;
  /**
   * Zeilen vor jedem Match anzeigen
   */
  '-B'?: number;
  /**
   * Zeilen nach jedem Match anzeigen
   */
  '-A'?: number;
  /**
   * Zeilen vor und nach jedem Match anzeigen
   */
  '-C'?: number;
  /**
   * Begrenzen Sie die Ausgabe auf die ersten N Zeilen/Einträge
   */
  head_limit?: number;
  /**
   * Aktivieren Sie den mehrzeiligen Modus
   */
  multiline?: boolean;
}
```

Leistungsstarkes Suchtool basierend auf ripgrep mit Regex-Unterstützung.

### KillBash

**Tool-Name:** `KillBash`

```typescript
interface KillShellInput {
  /**
   * Die ID der zu beendenden Hintergrund-Shell
   */
  shell_id: string;
}
```

Beendet eine laufende Hintergrund-Bash-Shell nach ihrer ID.

### NotebookEdit

**Tool-Name:** `NotebookEdit`

```typescript
interface NotebookEditInput {
  /**
   * Der absolute Pfad zur Jupyter-Notebook-Datei
   */
  notebook_path: string;
  /**
   * Die ID der zu bearbeitenden Zelle
   */
  cell_id?: string;
  /**
   * Die neue Quelle für die Zelle
   */
  new_source: string;
  /**
   * Der Typ der Zelle (Code oder Markdown)
   */
  cell_type?: 'code' | 'markdown';
  /**
   * Der Bearbeitungstyp (Ersetzen, Einfügen, Löschen)
   */
  edit_mode?: 'replace' | 'insert' | 'delete';
}
```

Bearbeitet Zellen in Jupyter-Notebook-Dateien.

### WebFetch

**Tool-Name:** `WebFetch`

```typescript
interface WebFetchInput {
  /**
   * Die URL, von der Inhalte abgerufen werden sollen
   */
  url: string;
  /**
   * Die Aufforderung, die auf den abgerufenen Inhalt angewendet werden soll
   */
  prompt: string;
}
```

Ruft Inhalte von einer URL ab und verarbeitet sie mit einem KI-Modell.

### WebSearch

**Tool-Name:** `WebSearch`

```typescript
interface WebSearchInput {
  /**
   * Die zu verwendende Suchabfrage
   */
  query: string;
  /**
   * Nur Ergebnisse von diesen Domänen einschließen
   */
  allowed_domains?: string[];
  /**
   * Ergebnisse von diesen Domänen niemals einschließen
   */
  blocked_domains?: string[];
}
```

Durchsucht das Web und gibt formatierte Ergebnisse zurück.

### TodoWrite

**Tool-Name:** `TodoWrite`

```typescript
interface TodoWriteInput {
  /**
   * Die aktualisierte Todo-Liste
   */
  todos: Array<{
    /**
     * Die Aufgabenbeschreibung
     */
    content: string;
    /**
     * Der Aufgabenstatus
     */
    status: 'pending' | 'in_progress' | 'completed';
    /**
     * Aktive Form der Aufgabenbeschreibung
     */
    activeForm: string;
  }>;
}
```

Erstellt und verwaltet eine strukturierte Aufgabenliste zur Verfolgung des Fortschritts.

### ExitPlanMode

**Tool-Name:** `ExitPlanMode`

```typescript
interface ExitPlanModeInput {
  /**
   * Der Plan, der vom Benutzer zur Genehmigung ausgeführt werden soll
   */
  plan: string;
}
```

Beendet den Planungsmodus und fordert den Benutzer auf, den Plan zu genehmigen.

### ListMcpResources

**Tool-Name:** `ListMcpResources`

```typescript
interface ListMcpResourcesInput {
  /**
   * Optionaler Servername zum Filtern von Ressourcen
   */
  server?: string;
}
```

Listet verfügbare MCP-Ressourcen von verbundenen Servern auf.

### ReadMcpResource

**Tool-Name:** `ReadMcpResource`

```typescript
interface ReadMcpResourceInput {
  /**
   * Der MCP-Servername
   */
  server: string;
  /**
   * Die zu lesende Ressourcen-URI
   */
  uri: string;
}
```

Liest eine bestimmte MCP-Ressource von einem Server.

## Tool-Ausgabetypen

Dokumentation von Ausgabeschemas für alle integrierten Claude Code-Tools. Diese Typen stellen die tatsächlichen Antwortdaten dar, die von jedem Tool zurückgegeben werden.

### `ToolOutput`

**Hinweis:** Dies ist ein reiner Dokumentationstyp zur Verdeutlichung. Er stellt die Union aller Tool-Ausgabetypen dar.

```typescript
type ToolOutput = 
  | TaskOutput
  | BashOutput
  | BashOutputToolOutput
  | EditOutput
  | ReadOutput
  | WriteOutput
  | GlobOutput
  | GrepOutput
  | KillBashOutput
  | NotebookEditOutput
  | WebFetchOutput
  | WebSearchOutput
  | TodoWriteOutput
  | ExitPlanModeOutput
  | ListMcpResourcesOutput
  | ReadMcpResourceOutput;
```

### Task

**Tool-Name:** `Task`

```typescript
interface TaskOutput {
  /**
   * Endgültige Ergebnisnachricht vom Subagenten
   */
  result: string;
  /**
   * Token-Nutzungsstatistiken
   */
  usage?: {
    input_tokens: number;
    output_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
  };
  /**
   * Gesamtkosten in USD
   */
  total_cost_usd?: number;
  /**
   * Ausführungsdauer in Millisekunden
   */
  duration_ms?: number;
}
```

Gibt das endgültige Ergebnis vom Subagenten nach Abschluss der delegierten Aufgabe zurück.

### Bash

**Tool-Name:** `Bash`

```typescript
interface BashOutput {
  /**
   * Kombinierte stdout- und stderr-Ausgabe
   */
  output: string;
  /**
   * Exit-Code des Befehls
   */
  exitCode: number;
  /**
   * Ob der Befehl aufgrund eines Timeouts beendet wurde
   */
  killed?: boolean;
  /**
   * Shell-ID für Hintergrundprozesse
   */
  shellId?: string;
}
```

Gibt Befehlsausgabe mit Exit-Status zurück. Hintergrund-Befehle werden sofort mit einer shellId zurückgegeben.

### BashOutput

**Tool-Name:** `BashOutput`

```typescript
interface BashOutputToolOutput {
  /**
   * Neue Ausgabe seit der letzten Überprüfung
   */
  output: string;
  /**
   * Aktueller Shell-Status
   */
  status: 'running' | 'completed' | 'failed';
  /**
   * Exit-Code (wenn abgeschlossen)
   */
  exitCode?: number;
}
```

Gibt inkrementelle Ausgabe von Hintergrund-Shells zurück.

### Edit

**Tool-Name:** `Edit`

```typescript
interface EditOutput {
  /**
   * Bestätigungsnachricht
   */
  message: string;
  /**
   * Anzahl der vorgenommenen Ersetzungen
   */
  replacements: number;
  /**
   * Dateipfad, der bearbeitet wurde
   */
  file_path: string;
}
```

Gibt Bestätigung erfolgreicher Bearbeitungen mit Ersetzungsanzahl zurück.

### Read

**Tool-Name:** `Read`

```typescript
type ReadOutput = 
  | TextFileOutput
  | ImageFileOutput
  | PDFFileOutput
  | NotebookFileOutput;

interface TextFileOutput {
  /**
   * Dateiinhalt mit Zeilennummern
   */
  content: string;
  /**
   * Gesamtzahl der Zeilen in der Datei
   */
  total_lines: number;
  /**
   * Tatsächlich zurückgegebene Zeilen
   */
  lines_returned: number;
}

interface ImageFileOutput {
  /**
   * Base64-codierte Bilddaten
   */
  image: string;
  /**
   * Bild-MIME-Typ
   */
  mime_type: string;
  /**
   * Dateigröße in Bytes
   */
  file_size: number;
}

interface PDFFileOutput {
  /**
   * Array von Seiteninhalten
   */
  pages: Array<{
    page_number: number;
    text?: string;
    images?: Array<{
      image: string;
      mime_type: string;
    }>;
  }>;
  /**
   * Gesamtzahl der Seiten
   */
  total_pages: number;
}

interface NotebookFileOutput {
  /**
   * Jupyter-Notebook-Zellen
   */
  cells: Array<{
    cell_type: 'code' | 'markdown';
    source: string;
    outputs?: any[];
    execution_count?: number;
  }>;
  /**
   * Notebook-Metadaten
   */
  metadata?: Record<string, any>;
}
```

Gibt Dateiinhalte in einem für den Dateityp geeigneten Format zurück.

### Write

**Tool-Name:** `Write`

```typescript
interface WriteOutput {
  /**
   * Erfolgsmeldung
   */
  message: string;
  /**
   * Anzahl der geschriebenen Bytes
   */
  bytes_written: number;
  /**
   * Dateipfad, der geschrieben wurde
   */
  file_path: string;
}
```

Gibt Bestätigung nach erfolgreichem Schreiben der Datei zurück.

### Glob

**Tool-Name:** `Glob`

```typescript
interface GlobOutput {
  /**
   * Array von übereinstimmenden Dateipfaden
   */
  matches: string[];
  /**
   * Anzahl der gefundenen Übereinstimmungen
   */
  count: number;
  /**
   * Verwendetes Suchverzeichnis
   */
  search_path: string;
}
```

Gibt Dateipfade zurück, die dem Glob-Muster entsprechen, sortiert nach Änderungszeit.

### Grep

**Tool-Name:** `Grep`

```typescript
type GrepOutput = 
  | GrepContentOutput
  | GrepFilesOutput
  | GrepCountOutput;

interface GrepContentOutput {
  /**
   * Übereinstimmende Zeilen mit Kontext
   */
  matches: Array<{
    file: string;
    line_number?: number;
    line: string;
    before_context?: string[];
    after_context?: string[];
  }>;
  /**
   * Gesamtzahl der Übereinstimmungen
   */
  total_matches: number;
}

interface GrepFilesOutput {
  /**
   * Dateien mit Übereinstimmungen
   */
  files: string[];
  /**
   * Anzahl der Dateien mit Übereinstimmungen
   */
  count: number;
}

interface GrepCountOutput {
  /**
   * Übereinstimmungszahlen pro Datei
   */
  counts: Array<{
    file: string;
    count: number;
  }>;
  /**
   * Gesamtübereinstimmungen in allen Dateien
   */
  total: number;
}
```

Gibt Suchergebnisse in dem durch output_mode angegebenen Format zurück.

### KillBash

**Tool-Name:** `KillBash`

```typescript
interface KillBashOutput {
  /**
   * Erfolgsmeldung
   */
  message: string;
  /**
   * ID der beendeten Shell
   */
  shell_id: string;
}
```

Gibt Bestätigung nach Beendigung der Hintergrund-Shell zurück.

### NotebookEdit

**Tool-Name:** `NotebookEdit`

```typescript
interface NotebookEditOutput {
  /**
   * Erfolgsmeldung
   */
  message: string;
  /**
   * Typ der durchgeführten Bearbeitung
   */
  edit_type: 'replaced' | 'inserted' | 'deleted';
  /**
   * Zellen-ID, die betroffen war
   */
  cell_id?: string;
  /**
   * Gesamtzellen im Notebook nach Bearbeitung
   */
  total_cells: number;
}
```

Gibt Bestätigung nach Änderung des Jupyter-Notebooks zurück.

### WebFetch

**Tool-Name:** `WebFetch`

```typescript
interface WebFetchOutput {
  /**
   * Antwort des KI-Modells auf die Aufforderung
   */
  response: string;
  /**
   * URL, die abgerufen wurde
   */
  url: string;
  /**
   * Endgültige URL nach Umleitungen
   */
  final_url?: string;
  /**
   * HTTP-Statuscode
   */
  status_code?: number;
}
```

Gibt die Analyse des abgerufenen Web-Inhalts durch die KI zurück.

### WebSearch

**Tool-Name:** `WebSearch`

```typescript
interface WebSearchOutput {
  /**
   * Suchergebnisse
   */
  results: Array<{
    title: string;
    url: string;
    snippet: string;
    /**
     * Zusätzliche Metadaten, falls verfügbar
     */
    metadata?: Record<string, any>;
  }>;
  /**
   * Gesamtzahl der Ergebnisse
   */
  total_results: number;
  /**
   * Die gesuchte Abfrage
   */
  query: string;
}
```

Gibt formatierte Suchergebnisse aus dem Web zurück.

### TodoWrite

**Tool-Name:** `TodoWrite`

```typescript
interface TodoWriteOutput {
  /**
   * Erfolgsmeldung
   */
  message: string;
  /**
   * Aktuelle Todo-Statistiken
   */
  stats: {
    total: number;
    pending: number;
    in_progress: number;
    completed: number;
  };
}
```

Gibt Bestätigung mit aktuellen Aufgabenstatistiken zurück.

### ExitPlanMode

**Tool-Name:** `ExitPlanMode`

```typescript
interface ExitPlanModeOutput {
  /**
   * Bestätigungsnachricht
   */
  message: string;
  /**
   * Ob der Benutzer den Plan genehmigt hat
   */
  approved?: boolean;
}
```

Gibt Bestätigung nach Beendigung des Planungsmodus zurück.

### ListMcpResources

**Tool-Name:** `ListMcpResources`

```typescript
interface ListMcpResourcesOutput {
  /**
   * Verfügbare Ressourcen
   */
  resources: Array<{
    uri: string;
    name: string;
    description?: string;
    mimeType?: string;
    server: string;
  }>;
  /**
   * Gesamtzahl der Ressourcen
   */
  total: number;
}
```

Gibt eine Liste der verfügbaren MCP-Ressourcen zurück.

### ReadMcpResource

**Tool name:** `ReadMcpResource`

```typescript
interface ReadMcpResourceOutput {
  /**
   * Resource contents
   */
  contents: Array<{
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
  }>;
  /**
   * Server that provided the resource
   */
  server: string;
}
```

Gibt den Inhalt der angeforderten MCP-Ressource zurück.

## Berechtigungstypen

### `PermissionUpdate`

Operationen zum Aktualisieren von Berechtigungen.

```typescript
type PermissionUpdate = 
  | {
      type: 'addRules';
      rules: PermissionRuleValue[];
      behavior: PermissionBehavior;
      destination: PermissionUpdateDestination;
    }
  | {
      type: 'replaceRules';
      rules: PermissionRuleValue[];
      behavior: PermissionBehavior;
      destination: PermissionUpdateDestination;
    }
  | {
      type: 'removeRules';
      rules: PermissionRuleValue[];
      behavior: PermissionBehavior;
      destination: PermissionUpdateDestination;
    }
  | {
      type: 'setMode';
      mode: PermissionMode;
      destination: PermissionUpdateDestination;
    }
  | {
      type: 'addDirectories';
      directories: string[];
      destination: PermissionUpdateDestination;
    }
  | {
      type: 'removeDirectories';
      directories: string[];
      destination: PermissionUpdateDestination;
    }
```

### `PermissionBehavior`

```typescript
type PermissionBehavior = 'allow' | 'deny' | 'ask';
```

### `PermissionUpdateDestination`

```typescript
type PermissionUpdateDestination = 
  | 'userSettings'     // Globale Benutzereinstellungen
  | 'projectSettings'  // Projekteinstellungen pro Verzeichnis
  | 'localSettings'    // Gitignored lokale Einstellungen
  | 'session'          // Nur aktuelle Sitzung
```

### `PermissionRuleValue`

```typescript
type PermissionRuleValue = {
  toolName: string;
  ruleContent?: string;
}
```

## Weitere Typen

### `ApiKeySource`

```typescript
type ApiKeySource = 'user' | 'project' | 'org' | 'temporary';
```

### `ConfigScope`

```typescript
type ConfigScope = 'local' | 'user' | 'project';
```

### `NonNullableUsage`

Eine Version von [`Usage`](#usage) mit allen nullable Feldern, die nicht nullable gemacht wurden.

```typescript
type NonNullableUsage = {
  [K in keyof Usage]: NonNullable<Usage[K]>;
}
```

### `Usage`

Token-Nutzungsstatistiken (von `@anthropic-ai/sdk`).

```typescript
type Usage = {
  input_tokens: number | null;
  output_tokens: number | null;
  cache_creation_input_tokens?: number | null;
  cache_read_input_tokens?: number | null;
}
```

### `CallToolResult`

MCP-Tool-Ergebnistyp (von `@modelcontextprotocol/sdk/types.js`).

```typescript
type CallToolResult = {
  content: Array<{
    type: 'text' | 'image' | 'resource';
    // Additional fields vary by type
  }>;
  isError?: boolean;
}
```

### `AbortError`

Benutzerdefinierte Fehlerklasse für Abbruchoperationen.

```typescript
class AbortError extends Error {}
```

## Siehe auch

- [SDK-Übersicht](/docs/de/agent-sdk/overview) - Allgemeine SDK-Konzepte
- [Python SDK-Referenz](/docs/de/agent-sdk/python) - Python SDK-Dokumentation
- [CLI-Referenz](https://code.claude.com/docs/de/cli-reference) - Befehlszeilenschnittstelle
- [Häufige Workflows](https://code.claude.com/docs/de/common-workflows) - Schritt-für-Schritt-Anleitungen
