/**
 * Session management route handlers.
 */

import { Router, Request, Response } from 'express';
import type { SessionStore } from '../store.js';

export function createSessionRoutes(store: SessionStore): Router {
  const router = Router();

  /**
   * GET /sessions
   * List stored sessions.
   */
  router.get('/', (req: Request, res: Response) => {
    const type = req.query.type as 'claude' | 'codex' | undefined;
    const limit = parseInt(req.query.limit as string || '100', 10);

    const sessions = store.list(type, limit);
    res.json({ sessions });
  });

  /**
   * GET /sessions/:id
   * Get a specific session.
   */
  router.get('/:id', (req: Request, res: Response) => {
    const session = store.get(req.params.id);

    if (!session) {
      res.status(404).json({ error: 'Session not found' });
      return;
    }

    res.json({ session });
  });

  /**
   * DELETE /sessions/:id
   * Delete a session.
   */
  router.delete('/:id', (req: Request, res: Response) => {
    const success = store.delete(req.params.id);
    res.json({ success });
  });

  /**
   * POST /sessions/prune
   * Delete old sessions.
   */
  router.post('/prune', (req: Request, res: Response) => {
    const days = parseInt(req.body.olderThanDays || '30', 10);
    const deleted = store.prune(days);
    res.json({ deleted });
  });

  return router;
}
