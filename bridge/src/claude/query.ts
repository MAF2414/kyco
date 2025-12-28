/**
 * Claude Agent SDK query execution
 */

import { query, type Query } from '@anthropic-ai/claude-agent-sdk';
import type { ClaudeQueryRequest, BridgeEvent, PermissionMode } from '../types.js';
import type { SessionStore } from '../store.js';
import { buildClaudePrompt } from './helpers.js';
import { cleanupSessionApprovals } from './approvals.js';
import { buildQueryOptions } from './options-builder.js';
import { createCanUseToolCallback } from './can-use-tool.js';
import type { EventEmitter } from './types.js';

/** Active queries that can be interrupted */
const activeQueries = new Map<string, Query>();

/** Current event emitter for the active query */
let currentEventEmitter: EventEmitter | null = null;

/**
 * Execute a Claude query and stream events back
 */
export async function* executeClaudeQuery(
  request: ClaudeQueryRequest,
  store: SessionStore,
  _kycoCallbackUrl?: string,
): AsyncGenerator<BridgeEvent> {
  let sessionId = request.sessionId || '';
  const startTime = Date.now();
  let totalInputTokens = 0;
  let totalOutputTokens = 0;
  let costUsd = 0;

  const eventQueue: BridgeEvent[] = [];
  let resolveNext: (() => void) | null = null;
  let sessionCompleted = false;

  const emitEvent = (event: BridgeEvent) => {
    eventQueue.push(event);
    if (resolveNext) {
      resolveNext();
      resolveNext = null;
    }
  };

  currentEventEmitter = emitEvent;
  let q: Query | null = null;

  try {
    const options = buildQueryOptions(request, sessionId, emitEvent);
    options!.canUseTool = createCanUseToolCallback(sessionId, emitEvent);

    q = query({ prompt: buildClaudePrompt(request), options });

    const processStream = async () => {
      try {
        for await (const message of q!) {
          switch (message.type) {
            case 'system': {
              if (message.subtype === 'init') {
                sessionId = message.session_id;
                activeQueries.set(sessionId, q!);
                emitEvent({
                  type: 'session.start',
                  sessionId: message.session_id,
                  timestamp: Date.now(),
                  model: message.model,
                  tools: message.tools,
                });
              }
              break;
            }
            case 'assistant': {
              for (const block of message.message.content) {
                if (block.type === 'text') {
                  emitEvent({
                    type: 'text',
                    sessionId,
                    timestamp: Date.now(),
                    content: block.text,
                    partial: false,
                  });
                } else if (block.type === 'tool_use') {
                  emitEvent({
                    type: 'tool.use',
                    sessionId,
                    timestamp: Date.now(),
                    toolName: block.name,
                    toolInput: block.input as Record<string, unknown>,
                    toolUseId: block.id,
                  });
                }
              }
              break;
            }
            case 'user': {
              for (const block of message.message.content) {
                if (block.type === 'tool_result') {
                  const output = typeof block.content === 'string'
                    ? block.content
                    : JSON.stringify(block.content);
                  emitEvent({
                    type: 'tool.result',
                    sessionId,
                    timestamp: Date.now(),
                    toolUseId: block.tool_use_id,
                    success: !block.is_error,
                    output,
                  });
                }
              }
              break;
            }
            case 'result': {
              sessionCompleted = true;
              if (message.subtype === 'success') {
                totalInputTokens = message.usage.input_tokens;
                totalOutputTokens = message.usage.output_tokens;
                costUsd = message.total_cost_usd;
                emitEvent({
                  type: 'session.complete',
                  sessionId,
                  timestamp: Date.now(),
                  success: true,
                  result: message.result,
                  usage: {
                    inputTokens: totalInputTokens,
                    outputTokens: totalOutputTokens,
                    cacheReadTokens: message.usage.cache_read_input_tokens,
                    cacheWriteTokens: message.usage.cache_creation_input_tokens,
                  },
                  costUsd,
                  durationMs: Date.now() - startTime,
                });
              } else {
                emitEvent({
                  type: 'session.complete',
                  sessionId,
                  timestamp: Date.now(),
                  success: false,
                  usage: {
                    inputTokens: message.usage.input_tokens,
                    outputTokens: message.usage.output_tokens,
                  },
                  costUsd: message.total_cost_usd,
                  durationMs: Date.now() - startTime,
                });
              }
              break;
            }
          }
        }

        if (!sessionCompleted) {
          emitEvent({
            type: 'session.complete',
            sessionId,
            timestamp: Date.now(),
            success: true,
            usage: { inputTokens: totalInputTokens, outputTokens: totalOutputTokens },
            costUsd,
            durationMs: Date.now() - startTime,
          });
        }

        const existingSession = store.get(sessionId);
        store.upsert({
          id: sessionId,
          type: 'claude',
          createdAt: existingSession?.createdAt || startTime,
          lastActiveAt: Date.now(),
          cwd: request.cwd,
          turnCount: (existingSession?.turnCount || 0) + 1,
          totalTokens: (existingSession?.totalTokens || 0) + totalInputTokens + totalOutputTokens,
          totalCostUsd: (existingSession?.totalCostUsd || 0) + costUsd,
        });
      } catch (error) {
        emitEvent({
          type: 'error',
          sessionId,
          timestamp: Date.now(),
          message: error instanceof Error ? error.message : String(error),
        });
        emitEvent({
          type: 'session.complete',
          sessionId,
          timestamp: Date.now(),
          success: false,
          durationMs: Date.now() - startTime,
        });
      } finally {
        activeQueries.delete(sessionId);
        currentEventEmitter = null;
      }
    };

    const streamPromise = processStream();

    while (true) {
      if (eventQueue.length > 0) {
        const event = eventQueue.shift()!;
        yield event;
        if (event.type === 'session.complete') break;
      } else {
        await new Promise<void>(resolve => {
          resolveNext = resolve;
          streamPromise.then(() => {
            if (resolveNext) {
              resolveNext();
              resolveNext = null;
            }
          });
        });
        if (eventQueue.length === 0) break;
      }
    }

    if (q) {
      try {
        await q.interrupt();
      } catch {
        try {
          await q.return(undefined as never);
        } catch {
          // Ignore cleanup errors
        }
      }
    }

    const timeoutPromise = new Promise<void>(resolve => setTimeout(resolve, 2000));
    await Promise.race([streamPromise.catch(() => {}), timeoutPromise]);
  } catch (error) {
    yield {
      type: 'error',
      sessionId,
      timestamp: Date.now(),
      message: error instanceof Error ? error.message : String(error),
    };
    yield {
      type: 'session.complete',
      sessionId,
      timestamp: Date.now(),
      success: false,
      durationMs: Date.now() - startTime,
    };
    if (q) {
      try {
        await q.return(undefined as never);
      } catch {
        // Ignore cleanup errors
      }
    }
  }
}

/**
 * Interrupt an active query
 */
export async function interruptClaudeQuery(sessionId: string): Promise<boolean> {
  const q = activeQueries.get(sessionId);
  if (q) {
    cleanupSessionApprovals(sessionId);
    await q.interrupt();
    return true;
  }
  return false;
}

/**
 * Change permission mode for an active session
 */
export async function setClaudePermissionMode(sessionId: string, mode: PermissionMode): Promise<boolean> {
  const q = activeQueries.get(sessionId);
  if (q) {
    await q.setPermissionMode(mode);
    return true;
  }
  return false;
}
