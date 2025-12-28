/**
 * Claude Agent SDK route handlers.
 */

import { Router, Request, Response } from 'express';
import type { SessionStore } from '../store.js';
import type { BridgeEvent } from '../types.js';
import {
  ClaudeQueryRequestSchema,
  SetPermissionModeRequestSchema,
  ToolApprovalResponseSchema,
} from '../types.js';
import {
  executeClaudeQuery,
  interruptClaudeQuery,
  resolveToolApproval,
  getPendingApprovals,
  setClaudePermissionMode,
} from '../claude.js';

export function createClaudeRoutes(store: SessionStore, kycoCallbackUrl?: string): Router {
  const router = Router();

  /**
   * POST /claude/query
   * Start or continue a Claude session.
   * Streams NDJSON events back to the client.
   */
  router.post('/query', async (req: Request, res: Response) => {
    const parseResult = ClaudeQueryRequestSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json({
        error: 'Invalid request',
        details: parseResult.error.errors,
      });
      return;
    }

    const request = parseResult.data;

    res.setHeader('Content-Type', 'application/x-ndjson');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Connection', 'keep-alive');

    try {
      for await (const event of executeClaudeQuery(request, store, kycoCallbackUrl)) {
        res.write(JSON.stringify(event) + '\n');
        if (typeof (res as unknown as { flush?: () => void }).flush === 'function') {
          (res as unknown as { flush: () => void }).flush();
        }
      }
    } catch (error) {
      const errorEvent: BridgeEvent = {
        type: 'error',
        sessionId: request.sessionId || 'unknown',
        timestamp: Date.now(),
        message: error instanceof Error ? error.message : String(error),
      };
      res.write(JSON.stringify(errorEvent) + '\n');
    }

    res.end();
  });

  /**
   * POST /claude/interrupt/:sessionId
   * Interrupt a running Claude session.
   */
  router.post('/interrupt/:sessionId', async (req: Request, res: Response) => {
    const { sessionId } = req.params;
    const success = await interruptClaudeQuery(sessionId);
    res.json({ success, sessionId });
  });

  /**
   * POST /claude/set-permission-mode/:sessionId
   * Change permission mode for an active Claude session.
   */
  router.post('/set-permission-mode/:sessionId', async (req: Request, res: Response) => {
    const { sessionId } = req.params;

    const parseResult = SetPermissionModeRequestSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json({
        error: 'Invalid request',
        details: parseResult.error.errors,
      });
      return;
    }

    try {
      const success = await setClaudePermissionMode(sessionId, parseResult.data.permissionMode);
      res.json({ success, sessionId });
    } catch (error) {
      res.status(500).json({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  });

  /**
   * POST /claude/tool-approval
   * Resolve a pending tool approval request.
   */
  router.post('/tool-approval', (req: Request, res: Response) => {
    const parseResult = ToolApprovalResponseSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json({
        error: 'Invalid request',
        details: parseResult.error.errors,
      });
      return;
    }

    const success = resolveToolApproval(parseResult.data);
    res.json({ success });
  });

  /**
   * GET /claude/pending-approvals
   * Get list of pending tool approval requests.
   */
  router.get('/pending-approvals', (_req: Request, res: Response) => {
    const pending = getPendingApprovals();
    res.json({ pending });
  });

  return router;
}
