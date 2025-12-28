/**
 * Tool approval management for Claude queries
 */

import type { ToolApprovalResponse } from '../types.js';

/**
 * Pending tool approval requests
 * Key: requestId, Value: Promise resolver and metadata
 */
export const pendingApprovals = new Map<string, {
  resolve: (response: ToolApprovalResponse) => void;
  toolName: string;
  toolInput: Record<string, unknown>;
  sessionId: string;
  heartbeatInterval?: ReturnType<typeof setInterval>;
}>();

/**
 * Wait for KYCO to respond to a tool approval request
 *
 * IMPORTANT: No timeout! The session stays paused until the user explicitly
 * decides to allow or deny. This is intentional for security - we never want
 * to auto-deny and cause Claude to retry in a loop, nor auto-allow dangerous
 * operations. The user MUST make the decision.
 *
 * We send periodic heartbeat events to keep the HTTP connection alive while waiting.
 * If an AbortSignal is provided and fires, we auto-deny to prevent hanging.
 */
export function waitForApproval(
  requestId: string,
  toolName: string,
  toolInput: Record<string, unknown>,
  sessionId: string,
  emitHeartbeat?: () => void,
  signal?: AbortSignal,
): Promise<ToolApprovalResponse> {
  return new Promise((resolve) => {
    const heartbeatInterval = emitHeartbeat
      ? setInterval(() => {
          emitHeartbeat();
        }, 15000)
      : undefined;

    const cleanup = (response: ToolApprovalResponse) => {
      if (heartbeatInterval) {
        clearInterval(heartbeatInterval);
      }
      pendingApprovals.delete(requestId);
      resolve(response);
    };

    if (signal) {
      signal.addEventListener('abort', () => {
        cleanup({
          requestId,
          decision: 'deny',
          reason: 'Session aborted by SDK',
        });
      }, { once: true });
    }

    pendingApprovals.set(requestId, {
      resolve: cleanup,
      toolName,
      toolInput,
      sessionId,
      heartbeatInterval,
    });
  });
}

/**
 * Resolve a pending tool approval request
 */
export function resolveToolApproval(response: ToolApprovalResponse): boolean {
  const pending = pendingApprovals.get(response.requestId);
  if (pending) {
    if (pending.heartbeatInterval) {
      clearInterval(pending.heartbeatInterval);
    }
    pending.resolve(response);
    pendingApprovals.delete(response.requestId);
    return true;
  }
  return false;
}

/**
 * Get pending approval requests (for debugging/status)
 */
export function getPendingApprovals(): Array<{
  requestId: string;
  toolName: string;
  toolInput: Record<string, unknown>;
}> {
  return Array.from(pendingApprovals.entries()).map(([requestId, data]) => ({
    requestId,
    toolName: data.toolName,
    toolInput: data.toolInput,
  }));
}

/**
 * Clean up pending approvals for a session (used during interrupt)
 */
export function cleanupSessionApprovals(sessionId: string): void {
  for (const [requestId, approval] of pendingApprovals.entries()) {
    if (approval.sessionId === sessionId) {
      if (approval.heartbeatInterval) {
        clearInterval(approval.heartbeatInterval);
      }
      approval.resolve({
        requestId,
        decision: 'deny',
        reason: 'Session interrupted by user',
      });
      pendingApprovals.delete(requestId);
    }
  }
}
