/**
 * Permission callback for Claude tool usage
 */

import { v4 as uuidv4 } from 'uuid';
import type { ToolApprovalNeededEvent, HeartbeatEvent } from '../types.js';
import { waitForApproval } from './approvals.js';
import type { EventEmitter } from './types.js';

// SDK-compatible PermissionResult type (discriminated union)
export type PermissionResult =
  | { behavior: 'allow'; updatedInput: Record<string, unknown>; toolUseID?: string }
  | { behavior: 'deny'; message: string; interrupt?: boolean; toolUseID?: string };

export type CanUseToolCallback = (
  toolName: string,
  toolInput: Record<string, unknown>,
  callbackOptions?: { signal?: AbortSignal; suggestions?: unknown[]; toolUseID?: string },
) => Promise<PermissionResult>;

export function createCanUseToolCallback(
  sessionId: string,
  emitEvent: EventEmitter,
): CanUseToolCallback {
  return async (toolName, toolInput, callbackOptions) => {
    const requestId = uuidv4();

    if (callbackOptions?.signal?.aborted) {
      return { behavior: 'deny' as const, message: 'Session aborted' };
    }

    const approvalEvent: ToolApprovalNeededEvent = {
      type: 'tool.approval_needed',
      sessionId,
      timestamp: Date.now(),
      requestId,
      toolName,
      toolInput,
    };
    emitEvent(approvalEvent);

    const emitHeartbeat = () => {
      const heartbeatEvent: HeartbeatEvent = {
        type: 'heartbeat',
        sessionId,
        timestamp: Date.now(),
        pendingApprovalRequestId: requestId,
      };
      emitEvent(heartbeatEvent);
    };

    const response = await waitForApproval(
      requestId,
      toolName,
      toolInput,
      sessionId,
      emitHeartbeat,
      callbackOptions?.signal,
    );

    if (response.decision === 'allow') {
      return {
        behavior: 'allow',
        updatedInput: response.modifiedInput || toolInput,
      } as const;
    }
    return {
      behavior: 'deny',
      message: response.reason || 'User denied permission',
    } as const;
  };
}
