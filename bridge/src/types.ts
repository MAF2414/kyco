/**
 * KYCO SDK Bridge - Type Definitions
 *
 * These types define the communication protocol between KYCO (Rust) and the SDK Bridge (Node.js).
 *
 * This module re-exports all types from:
 * - ./schemas.ts - Zod validation schemas
 * - ./events.ts - Event type definitions
 * - ./config.ts - Configuration & session types
 */

// ============================================================================
// Simple Type Aliases
// ============================================================================

/** Permission mode for Claude sessions */
export type PermissionMode = 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan' | 'delegate' | 'dontAsk';

/** Tool approval decision from KYCO */
export type ToolDecision = 'allow' | 'deny' | 'ask';

/** Codex approval policy options */
export type CodexApprovalPolicy = 'never' | 'on-request' | 'on-failure' | 'untrusted';

// ============================================================================
// Re-exports: Schemas (Zod validation)
// ============================================================================

export {
  // Hook schemas
  HookEventSchema,
  ClaudeHooksConfigSchema,
  // MCP & Plugin schemas
  McpServerConfigSchema,
  ImageContentSchema,
  ClaudePluginSchema,
  AgentConfigSchema,
  // Query schemas
  ClaudeQueryRequestSchema,
  SetPermissionModeRequestSchema,
  CodexQueryRequestSchema,
  // Tool approval schema
  ToolApprovalResponseSchema,
} from './schemas.js';

export type {
  HookEvent,
  ClaudeHooksConfig,
  ImageContent,
  ClaudePlugin,
  AgentConfig,
  ClaudeQueryRequest,
  SetPermissionModeRequest,
  CodexQueryRequest,
  ToolApprovalResponse,
} from './schemas.js';

// ============================================================================
// Re-exports: Events (response types)
// ============================================================================

export type {
  BaseEvent,
  SessionStartEvent,
  SessionCompleteEvent,
  TextEvent,
  ReasoningEvent,
  ErrorEvent,
  ToolUseEvent,
  ToolResultEvent,
  ToolApprovalNeededEvent,
  HookPreToolUseEvent,
  HeartbeatEvent,
  BridgeEvent,
} from './events.js';

// ============================================================================
// Re-exports: Config & Session
// ============================================================================

export { DEFAULT_CONFIG } from './config.js';

export type {
  StoredSession,
  ToolApprovalRequest,
  BridgeConfig,
} from './config.js';
