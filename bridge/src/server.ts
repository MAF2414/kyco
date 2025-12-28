/**
 * KYCO SDK Bridge Server
 *
 * HTTP server that exposes Claude Agent SDK and Codex SDK functionality to KYCO.
 * Communication uses newline-delimited JSON (NDJSON) for streaming.
 */

import express, { Request, Response } from 'express';
import { BridgeConfig, DEFAULT_CONFIG } from './types.js';
import { SessionStore } from './store.js';
import { listCodexThreads } from './codex.js';
import { createClaudeRoutes, createCodexRoutes, createSessionRoutes } from './routes/index.js';

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
// Mount Route Modules
// ============================================================================

app.use('/claude', createClaudeRoutes(store, config.kycoCallbackUrl));
app.use('/codex', createCodexRoutes(store));
app.use('/sessions', createSessionRoutes(store));

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
