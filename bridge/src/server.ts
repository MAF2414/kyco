/**
 * KYCO SDK Bridge Server
 *
 * HTTP server that exposes Claude Agent SDK and Codex SDK functionality to KYCO.
 * Communication uses newline-delimited JSON (NDJSON) for streaming.
 */

import express, { Request, Response } from 'express';
import { randomUUID } from 'crypto';
import {
  BridgeConfig,
  DEFAULT_CONFIG,
  ClaudeQueryRequestSchema,
  CodexQueryRequestSchema,
  SetPermissionModeRequestSchema,
  ToolApprovalResponseSchema,
  type BridgeEvent,
} from './types.js';
import { SessionStore } from './store.js';
import {
  executeClaudeQuery,
  interruptClaudeQuery,
  resolveToolApproval,
  getPendingApprovals,
  setClaudePermissionMode,
} from './claude.js';
import {
  executeCodexQuery,
  executeCodexQueryWithSchema,
  listCodexThreads,
  clearCodexThread,
  interruptCodexThread,
} from './codex.js';

// Load config from environment or use defaults
const config: BridgeConfig = {
  port: parseInt(process.env.KYCO_BRIDGE_PORT || String(DEFAULT_CONFIG.port), 10),
  host: process.env.KYCO_BRIDGE_HOST || DEFAULT_CONFIG.host,
  dbPath: process.env.KYCO_BRIDGE_DB || DEFAULT_CONFIG.dbPath,
  kycoCallbackUrl: process.env.KYCO_CALLBACK_URL,
};

// Initialize store (SQLite-backed, auto-migrates legacy JSON if present)
const store = new SessionStore(config.dbPath);

// Create Express app
const app = express();
const jsonBodyLimit = process.env.KYCO_BRIDGE_JSON_LIMIT || '25mb';
app.use(express.json({ limit: jsonBodyLimit }));

// ============================================================================
// Health & Status
// ============================================================================

app.get('/health', (_req: Request, res: Response) => {
  res.json({
    status: 'ok',
    version: '0.1.0',
    timestamp: Date.now(),
  });
});

app.get('/status', (_req: Request, res: Response) => {
  const claudeSessions = store.list('claude', 10);
  const codexThreads = listCodexThreads();

  res.json({
    activeSessions: {
      claude: claudeSessions.length,
      codex: codexThreads.length,
    },
    recentClaudeSessions: claudeSessions.map(s => ({
      id: s.id,
      turnCount: s.turnCount,
      lastActive: new Date(s.lastActiveAt).toISOString(),
    })),
    activeCodexThreads: codexThreads,
  });
});

// ============================================================================
// Claude Agent SDK Endpoints
// ============================================================================

/**
 * POST /claude/query
 *
 * Start or continue a Claude session.
 * Streams NDJSON events back to the client.
 */
app.post('/claude/query', async (req: Request, res: Response) => {
  // Validate request
  const parseResult = ClaudeQueryRequestSchema.safeParse(req.body);
  if (!parseResult.success) {
    res.status(400).json({
      error: 'Invalid request',
      details: parseResult.error.errors,
    });
    return;
  }

  const request = parseResult.data;

  // Set up streaming response
  res.setHeader('Content-Type', 'application/x-ndjson');
  res.setHeader('Cache-Control', 'no-cache');
  res.setHeader('Connection', 'keep-alive');

  try {
    // Stream events
    for await (const event of executeClaudeQuery(request, store, config.kycoCallbackUrl)) {
      res.write(JSON.stringify(event) + '\n');

      // Flush after each event for real-time streaming
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
 *
 * Interrupt a running Claude session.
 */
app.post('/claude/interrupt/:sessionId', async (req: Request, res: Response) => {
  const { sessionId } = req.params;
  const success = await interruptClaudeQuery(sessionId);

  res.json({ success, sessionId });
});

/**
 * POST /claude/set-permission-mode/:sessionId
 *
 * Change permission mode for an active Claude session.
 */
app.post('/claude/set-permission-mode/:sessionId', async (req: Request, res: Response) => {
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
 *
 * Resolve a pending tool approval request.
 */
app.post('/claude/tool-approval', (req: Request, res: Response) => {
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
 *
 * Get list of pending tool approval requests.
 * KYCO can poll this to check for new approval requests.
 */
app.get('/claude/pending-approvals', (_req: Request, res: Response) => {
  const pending = getPendingApprovals();
  res.json({ pending });
});

// ============================================================================
// Codex SDK Endpoints
// ============================================================================

/**
 * POST /codex/query
 *
 * Start or continue a Codex thread.
 * Streams NDJSON events back to the client.
 */
app.post('/codex/query', async (req: Request, res: Response) => {
  // Validate request
  const parseResult = CodexQueryRequestSchema.safeParse(req.body);
  if (!parseResult.success) {
    res.status(400).json({
      error: 'Invalid request',
      details: parseResult.error.errors,
    });
    return;
  }

  const request = parseResult.data;

  // Set up streaming response
  res.setHeader('Content-Type', 'application/x-ndjson');
  res.setHeader('Cache-Control', 'no-cache');
  res.setHeader('Connection', 'keep-alive');

  try {
    // Stream events
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
 *
 * Run a Codex query with structured output schema.
 */
app.post('/codex/query-structured', async (req: Request, res: Response) => {
  const { prompt, cwd, outputSchema } = req.body;

  if (!prompt || !cwd || !outputSchema) {
    res.status(400).json({
      error: 'Missing required fields: prompt, cwd, outputSchema',
    });
    return;
  }

  // Set up streaming response
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
 *
 * List active Codex threads.
 */
app.get('/codex/threads', (_req: Request, res: Response) => {
  res.json({ threads: listCodexThreads() });
});

/**
 * DELETE /codex/threads/:threadId
 *
 * Clear a Codex thread from memory.
 */
app.delete('/codex/threads/:threadId', (req: Request, res: Response) => {
  const { threadId } = req.params;
  const success = clearCodexThread(threadId);
  res.json({ success, threadId });
});

/**
 * POST /codex/interrupt/:threadId
 *
 * Interrupt a running Codex turn.
 */
app.post('/codex/interrupt/:threadId', (req: Request, res: Response) => {
  const { threadId } = req.params;
  const success = interruptCodexThread(threadId);
  res.json({ success, threadId });
});

// ============================================================================
// Session Management
// ============================================================================

/**
 * GET /sessions
 *
 * List stored sessions.
 */
app.get('/sessions', (req: Request, res: Response) => {
  const type = req.query.type as 'claude' | 'codex' | undefined;
  const limit = parseInt(req.query.limit as string || '100', 10);

  const sessions = store.list(type, limit);
  res.json({ sessions });
});

/**
 * GET /sessions/:id
 *
 * Get a specific session.
 */
app.get('/sessions/:id', (req: Request, res: Response) => {
  const session = store.get(req.params.id);

  if (!session) {
    res.status(404).json({ error: 'Session not found' });
    return;
  }

  res.json({ session });
});

/**
 * DELETE /sessions/:id
 *
 * Delete a session.
 */
app.delete('/sessions/:id', (req: Request, res: Response) => {
  const success = store.delete(req.params.id);
  res.json({ success });
});

/**
 * POST /sessions/prune
 *
 * Delete old sessions.
 */
app.post('/sessions/prune', (req: Request, res: Response) => {
  const days = parseInt(req.body.olderThanDays || '30', 10);
  const deleted = store.prune(days);
  res.json({ deleted });
});

// ============================================================================
// Startup
// ============================================================================

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down...');
  store.close();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log('\nShutting down...');
  store.close();
  process.exit(0);
});

// Start server
app.listen(config.port, config.host, () => {
  console.log(`KYCO SDK Bridge v0.1.0`);
  console.log(`Listening on http://${config.host}:${config.port}`);
  console.log(`Database: ${config.dbPath}`);
  if (config.kycoCallbackUrl) {
    console.log(`KYCO Callback URL: ${config.kycoCallbackUrl}`);
  }
  console.log('');
  console.log('Endpoints:');
  console.log('  POST /claude/query       - Start/continue Claude session');
  console.log('  POST /claude/interrupt   - Interrupt Claude session');
  console.log('  POST /codex/query        - Start/continue Codex thread');
  console.log('  GET  /sessions           - List sessions');
  console.log('  GET  /health             - Health check');
});
