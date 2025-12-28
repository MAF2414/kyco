/**
 * Execute a Codex query with structured output schema
 */

import { v4 as uuidv4 } from 'uuid';
import type { CodexQueryRequest, BridgeEvent } from '../types.js';
import type { SessionStore } from '../store.js';
import { getCodexInstance } from './instance.js';
import { tryParseJsonObject, buildCodexInput } from './utils.js';

export async function* executeCodexQueryWithSchema(
  request: CodexQueryRequest & { outputSchema: Record<string, unknown> },
  store: SessionStore,
): AsyncGenerator<BridgeEvent> {
  const startTime = Date.now();
  const codex = getCodexInstance();
  const thread = codex.startThread();
  const threadId = uuidv4();

  try {
    yield {
      type: 'session.start',
      sessionId: threadId,
      timestamp: Date.now(),
      model: 'codex',
      tools: ['bash', 'read', 'write', 'search'],
    };

    // Run with schema - this returns structured output directly
    const { input, cleanup } = await buildCodexInput(request);
    try {
      const turn = await thread.run(input, {
        outputSchema: request.outputSchema,
      });

      const parsed = tryParseJsonObject(turn.finalResponse);

      yield {
        type: 'text',
        sessionId: threadId,
        timestamp: Date.now(),
        content: turn.finalResponse,
        partial: false,
      };

      yield {
        type: 'session.complete',
        sessionId: threadId,
        timestamp: Date.now(),
        success: true,
        ...(parsed ? { result: parsed } : {}),
        durationMs: Date.now() - startTime,
      };

      // Store session
      store.upsert({
        id: threadId,
        type: 'codex',
        createdAt: startTime,
        lastActiveAt: Date.now(),
        cwd: request.cwd,
        turnCount: 1,
        totalTokens: 0,
        totalCostUsd: 0,
      });
    } finally {
      await cleanup();
    }

  } catch (error) {
    yield {
      type: 'error',
      sessionId: threadId,
      timestamp: Date.now(),
      message: error instanceof Error ? error.message : String(error),
    };

    yield {
      type: 'session.complete',
      sessionId: threadId,
      timestamp: Date.now(),
      success: false,
      durationMs: Date.now() - startTime,
    };
  }
}
