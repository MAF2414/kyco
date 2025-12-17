/**
 * KYCO SDK Bridge - Type Definitions
 *
 * These types define the communication protocol between KYCO (Rust) and the SDK Bridge (Node.js).
 */

import { z } from 'zod';

// ============================================================================
// Request Types
// ============================================================================

/** Permission mode for Claude sessions */
export type PermissionMode = 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan';

/** Tool approval decision from KYCO */
export type ToolDecision = 'allow' | 'deny' | 'ask';

// ============================================================================
// Hook Types (Claude SDK)
// ============================================================================

/** Supported Claude SDK hook events (subset used by the bridge) */
export const HookEventSchema = z.enum([
  'PreToolUse',
  'PostToolUse',
  'PostToolUseFailure',
  'Notification',
  'UserPromptSubmit',
  'SessionStart',
  'SessionEnd',
  'Stop',
  'SubagentStart',
  'SubagentStop',
  'PreCompact',
  'PermissionRequest',
]);

export type HookEvent = z.infer<typeof HookEventSchema>;

/** Hook configuration for the bridge (emits hook events to the NDJSON stream) */
export const ClaudeHooksConfigSchema = z.object({
  /** Hook events to enable. Currently only `PreToolUse` is emitted by the bridge. */
  events: z.array(HookEventSchema).optional(),
}).optional();

export type ClaudeHooksConfig = z.infer<typeof ClaudeHooksConfigSchema>;

/** Request to start or continue a Claude query */
const McpServerConfigSchema = z.object({
  /** Command to run the MCP server (e.g., "npx", "node", path to binary) */
  command: z.string(),
  /** Arguments to pass to the command */
  args: z.array(z.string()).optional(),
  /** Environment variables for the MCP server */
  env: z.record(z.string()).optional(),
  /** Optional working directory */
  cwd: z.string().optional(),
});

/** Base64-encoded image content to attach to a prompt. */
export const ImageContentSchema = z.object({
  /** Base64 data (no data URL prefix). */
  data: z.string().min(1),
  /** Media type (e.g., "image/png", "image/jpeg"). Defaults to "image/png". */
  mediaType: z.string().min(1).optional(),
});

export type ImageContent = z.infer<typeof ImageContentSchema>;

/** Claude SDK plugin definition (KYCO only supports local filesystem plugins) */
export const ClaudePluginSchema = z.object({
  type: z.literal('local'),
  path: z.string(),
});

export type ClaudePlugin = z.infer<typeof ClaudePluginSchema>;

/** Claude SDK subagent definition (aka "agents" option) */
export const AgentConfigSchema = z.object({
  /** Natural language description of when to use this agent */
  description: z.string(),
  /** The agent's system prompt */
  prompt: z.string(),
  /** Array of allowed tool names. If omitted, inherits all tools from parent */
  tools: z.array(z.string()).optional(),
  /** Array of tool names to explicitly disallow for this agent */
  disallowedTools: z.array(z.string()).optional(),
  /** Model alias (e.g., "sonnet", "opus", "haiku", "inherit") */
  model: z.enum(['sonnet', 'opus', 'haiku', 'inherit']).optional(),
  /** Experimental: critical reminder added to the system prompt */
  criticalSystemReminder_EXPERIMENTAL: z.string().optional(),
});

export type AgentConfig = z.infer<typeof AgentConfigSchema>;

export const ClaudeQueryRequestSchema = z.object({
  /** The prompt to send */
  prompt: z.string(),
  /** Optional images to attach to the prompt (currently supports a single image). */
  images: z.array(ImageContentSchema).max(1).optional(),
  /** Working directory for the agent */
  cwd: z.string(),
  /** Session ID to resume (optional) */
  sessionId: z.string().optional(),
  /** Fork the session instead of continuing it */
  forkSession: z.boolean().optional(),
  /** Permission mode */
  permissionMode: z.enum(['default', 'acceptEdits', 'bypassPermissions', 'plan']).optional(),
  /** Programmatically defined Claude subagents (Claude SDK only) */
  agents: z.record(AgentConfigSchema).optional(),
  /** Allowed tools (optional - if not set, all tools allowed) */
  allowedTools: z.array(z.string()).optional(),
  /** Disallowed tools */
  disallowedTools: z.array(z.string()).optional(),
  /** Environment variables to pass to the SDK process */
  env: z.record(z.string()).optional(),
  /** MCP servers to enable for this session (Claude SDK only) */
  mcpServers: z.record(McpServerConfigSchema).optional(),
  /** System prompt (append/replace depending on systemPromptMode) */
  systemPrompt: z.string().optional(),
  /** System prompt mode ("append" or "replace") */
  systemPromptMode: z.enum(['append', 'replace']).optional(),
  /** Which Claude Code settings sources to load (must include "project" for CLAUDE.md) */
  settingSources: z.array(z.enum(['user', 'project', 'local'])).optional(),
  /** Claude Agent SDK plugins to load (local filesystem allowlist) */
  plugins: z.array(ClaudePluginSchema).optional(),
  /** Maximum turns before stopping */
  maxTurns: z.number().optional(),
  /** Maximum thinking tokens for extended thinking (enables thinking if > 0) */
  maxThinkingTokens: z.number().optional(),
  /** Model to use (sonnet, opus, haiku) */
  model: z.string().optional(),
  /** Structured output schema (JSON Schema) */
  outputSchema: z.record(z.unknown()).optional(),
  /** KYCO callback URL for tool approvals */
  kycoCallbackUrl: z.string().optional(),
  /** Hook configuration (Claude SDK only) */
  hooks: ClaudeHooksConfigSchema,
});

export type ClaudeQueryRequest = z.infer<typeof ClaudeQueryRequestSchema>;

export const SetPermissionModeRequestSchema = z.object({
  permissionMode: z.enum(['default', 'acceptEdits', 'bypassPermissions', 'plan']),
});

export type SetPermissionModeRequest = z.infer<typeof SetPermissionModeRequestSchema>;

/** Request to start or continue a Codex thread */
export const CodexQueryRequestSchema = z.object({
  /** The prompt to send */
  prompt: z.string(),
  /** Optional images to attach to the prompt (currently supports a single image). */
  images: z.array(ImageContentSchema).max(1).optional(),
  /** Working directory */
  cwd: z.string(),
  /** Thread ID to resume (optional) */
  threadId: z.string().optional(),
  /** Sandbox mode */
  sandbox: z.enum(['read-only', 'workspace-write', 'danger-full-access']).optional(),
  /** Environment variables to pass to the Codex CLI process */
  env: z.record(z.string()).optional(),
  /** Structured output schema (JSON Schema) */
  outputSchema: z.record(z.unknown()).optional(),
  /** Skip the git repository check (for temp directories, non-git projects) */
  skipGitRepoCheck: z.boolean().optional(),
  /** Model to use (optional, uses Codex default if not specified) */
  model: z.string().optional(),
  /** Model reasoning effort level (controls how thorough Codex is) */
  effort: z.enum(['none', 'minimal', 'low', 'medium', 'high', 'xhigh']).optional(),
  /** Approval policy for tool use (default: 'never') */
  approvalPolicy: z.enum(['never', 'on-request', 'on-failure', 'untrusted']).optional(),
});

/** Codex approval policy options */
export type CodexApprovalPolicy = 'never' | 'on-request' | 'on-failure' | 'untrusted';

export type CodexQueryRequest = z.infer<typeof CodexQueryRequestSchema>;

/** Tool approval request sent to KYCO */
export interface ToolApprovalRequest {
  /** Unique ID for this approval request */
  requestId: string;
  /** Session/thread ID */
  sessionId: string;
  /** Tool name (e.g., 'Bash', 'Write', 'Edit') */
  toolName: string;
  /** Tool input parameters */
  toolInput: Record<string, unknown>;
}

/** Tool approval response from KYCO */
export const ToolApprovalResponseSchema = z.object({
  requestId: z.string(),
  decision: z.enum(['allow', 'deny', 'ask']),
  reason: z.string().optional(),
  /** Modified input (if KYCO wants to change parameters) */
  modifiedInput: z.record(z.unknown()).optional(),
});

export type ToolApprovalResponse = z.infer<typeof ToolApprovalResponseSchema>;

// ============================================================================
// Response/Event Types (streamed back to KYCO)
// ============================================================================

/** Base event structure */
export interface BaseEvent {
  type: string;
  sessionId: string;
  timestamp: number;
}

/** Session started event */
export interface SessionStartEvent extends BaseEvent {
  type: 'session.start';
  model: string;
  tools: string[];
}

/** Text output from the assistant */
export interface TextEvent extends BaseEvent {
  type: 'text';
  content: string;
  /** Whether this is a partial/streaming update */
  partial: boolean;
}

/** Tool use event */
export interface ToolUseEvent extends BaseEvent {
  type: 'tool.use';
  toolName: string;
  toolInput: Record<string, unknown>;
  toolUseId: string;
}

/** Tool result event */
export interface ToolResultEvent extends BaseEvent {
  type: 'tool.result';
  toolUseId: string;
  success: boolean;
  output: string;
  /** Files changed by this tool */
  filesChanged?: string[];
}

/** Error event */
export interface ErrorEvent extends BaseEvent {
  type: 'error';
  message: string;
  code?: string;
}

/** Session completed event */
export interface SessionCompleteEvent extends BaseEvent {
  type: 'session.complete';
  success: boolean;
  /** Final result (if structured output was requested) */
  result?: unknown;
  /** Usage statistics */
  usage?: {
    inputTokens: number;
    outputTokens: number;
    cacheReadTokens?: number;
    cacheWriteTokens?: number;
  };
  /** Cost in USD */
  costUsd?: number;
  /** Duration in milliseconds */
  durationMs: number;
}

/** Tool approval needed event (sent when KYCO callback is configured) */
export interface ToolApprovalNeededEvent extends BaseEvent {
  type: 'tool.approval_needed';
  requestId: string;
  toolName: string;
  toolInput: Record<string, unknown>;
}

/** Hook fired by the Claude SDK before executing a tool */
export interface HookPreToolUseEvent extends BaseEvent {
  type: 'hook.pre_tool_use';
  toolName: string;
  toolInput: Record<string, unknown>;
  toolUseId: string;
  /** Optional path to the transcript file on disk */
  transcriptPath?: string;
}

/** Heartbeat event to keep HTTP connection alive during tool approval waits */
export interface HeartbeatEvent extends BaseEvent {
  type: 'heartbeat';
  /** The pending approval request ID this heartbeat is for */
  pendingApprovalRequestId?: string;
}

/** Union of all event types */
export type BridgeEvent =
  | SessionStartEvent
  | TextEvent
  | ToolUseEvent
  | ToolResultEvent
  | ErrorEvent
  | SessionCompleteEvent
  | ToolApprovalNeededEvent
  | HookPreToolUseEvent
  | HeartbeatEvent;

// ============================================================================
// Session Store Types
// ============================================================================

/** Stored session metadata */
export interface StoredSession {
  id: string;
  type: 'claude' | 'codex';
  createdAt: number;
  lastActiveAt: number;
  cwd: string;
  /** Number of turns/messages in the session */
  turnCount: number;
  /** Total tokens used */
  totalTokens: number;
  /** Total cost in USD */
  totalCostUsd: number;
}

// ============================================================================
// Server Configuration
// ============================================================================

export interface BridgeConfig {
  /** Port to listen on */
  port: number;
  /** Host to bind to */
  host: string;
  /** Path to JSON file for session storage */
  dbPath: string;
  /** Default KYCO callback URL for tool approvals */
  kycoCallbackUrl?: string;
}

export const DEFAULT_CONFIG: BridgeConfig = {
  port: 17432,  // KYCO in leetspeak :)
  host: '127.0.0.1',
  dbPath: './kyco-sessions.json',
};
