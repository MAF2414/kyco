/**
 * KYCO SDK Bridge - Event Type Definitions
 *
 * Response/event types streamed back to KYCO via NDJSON.
 */

// ============================================================================
// Base Event
// ============================================================================

/** Base event structure */
export interface BaseEvent {
  type: string;
  sessionId: string;
  timestamp: number;
}

// ============================================================================
// Session Events
// ============================================================================

/** Session started event */
export interface SessionStartEvent extends BaseEvent {
  type: 'session.start';
  model: string;
  tools: string[];
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

// ============================================================================
// Content Events
// ============================================================================

/** Text output from the assistant */
export interface TextEvent extends BaseEvent {
  type: 'text';
  content: string;
  /** Whether this is a partial/streaming update */
  partial: boolean;
}

/** Error event */
export interface ErrorEvent extends BaseEvent {
  type: 'error';
  message: string;
  code?: string;
}

// ============================================================================
// Tool Events
// ============================================================================

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

/** Tool approval needed event (sent when KYCO callback is configured) */
export interface ToolApprovalNeededEvent extends BaseEvent {
  type: 'tool.approval_needed';
  requestId: string;
  toolName: string;
  toolInput: Record<string, unknown>;
}

// ============================================================================
// Hook Events
// ============================================================================

/** Hook fired by the Claude SDK before executing a tool */
export interface HookPreToolUseEvent extends BaseEvent {
  type: 'hook.pre_tool_use';
  toolName: string;
  toolInput: Record<string, unknown>;
  toolUseId: string;
  /** Optional path to the transcript file on disk */
  transcriptPath?: string;
}

// ============================================================================
// Utility Events
// ============================================================================

/** Heartbeat event to keep HTTP connection alive during tool approval waits */
export interface HeartbeatEvent extends BaseEvent {
  type: 'heartbeat';
  /** The pending approval request ID this heartbeat is for */
  pendingApprovalRequestId?: string;
}

// ============================================================================
// Union Type
// ============================================================================

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
