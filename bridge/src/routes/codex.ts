/**
 * Codex SDK route handlers.
 */

import { Router, Request, Response } from 'express';
import type { SessionStore } from '../store.js';
import type { BridgeEvent } from '../types.js';
import { CodexQueryRequestSchema } from '../types.js';
import {
  executeCodexQuery,
  executeCodexQueryWithSchema,
  listCodexThreads,
  clearCodexThread,
  interruptCodexThread,
} from '../codex.js';

export function createCodexRoutes(store: SessionStore): Router {
  const router = Router();

  /**
   * POST /codex/query
   * Start or continue a Codex thread.
   * Streams NDJSON events back to the client.
   */
  router.post('/query', async (req: Request, res: Response) => {
    const parseResult = CodexQueryRequestSchema.safeParse(req.body);
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
      for await (const event of executeCodexQuery(request, store)) {
        res.write(JSON.stringify(event) + '\n');
        if (typeof (res as unknown as { flush?: () => void }).flush === 'function') {
          (res as unknown as { flush: () => void }).flush();
        }
      }
    } catch (error) {
      const errorEvent: BridgeEvent = {
        type: 'error',
        sessionId: request.threadId || 'unknown',
        timestamp: Date.now(),
        message: error instanceof Error ? error.message : String(error),
      };
      res.write(JSON.stringify(errorEvent) + '\n');
    }

    res.end();
  });

  /**
   * POST /codex/query-structured
   * Run a Codex query with structured output schema.
   */
  router.post('/query-structured', async (req: Request, res: Response) => {
    const { prompt, cwd, outputSchema } = req.body;

    if (!prompt || !cwd || !outputSchema) {
      res.status(400).json({
        error: 'Missing required fields: prompt, cwd, outputSchema',
      });
      return;
    }

    res.setHeader('Content-Type', 'application/x-ndjson');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Connection', 'keep-alive');

    try {
      for await (const event of executeCodexQueryWithSchema({ prompt, cwd, outputSchema }, store)) {
        res.write(JSON.stringify(event) + '\n');
      }
    } catch (error) {
      const errorEvent: BridgeEvent = {
        type: 'error',
        sessionId: 'unknown',
        timestamp: Date.now(),
        message: error instanceof Error ? error.message : String(error),
      };
      res.write(JSON.stringify(errorEvent) + '\n');
    }

    res.end();
  });

  /**
   * GET /codex/threads
   * List active Codex threads.
   */
  router.get('/threads', (_req: Request, res: Response) => {
    res.json({ threads: listCodexThreads() });
  });

  /**
   * DELETE /codex/threads/:threadId
   * Clear a Codex thread from memory.
   */
  router.delete('/threads/:threadId', (req: Request, res: Response) => {
    const { threadId } = req.params;
    const success = clearCodexThread(threadId);
    res.json({ success, threadId });
  });

  /**
   * POST /codex/interrupt/:threadId
   * Interrupt a running Codex turn.
   */
  router.post('/interrupt/:threadId', (req: Request, res: Response) => {
    const { threadId } = req.params;
    const success = interruptCodexThread(threadId);
    res.json({ success, threadId });
  });

  return router;
}
