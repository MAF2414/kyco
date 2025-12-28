/**
 * KYCO SDK Bridge - Configuration & Session Types
 */

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
