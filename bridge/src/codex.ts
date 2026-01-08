/**
 * Codex SDK Integration
 *
 * Wraps the OpenAI Codex SDK to provide a streaming interface for KYCO.
 * Based on the samples in docs/codexSamples/
 */

import type { ThreadOptions, TurnOptions } from '@openai/codex-sdk';
import { v4 as uuidv4 } from 'uuid';
import type { CodexQueryRequest, BridgeEvent } from './types.js';
import { SessionStore } from './store.js';

// Re-export thread management functions
export {
  listCodexThreads,
  interruptCodexThread,
  clearCodexThread,
} from './codex/instance.js';

// Re-export schema query function
export { executeCodexQueryWithSchema } from './codex/query-with-schema.js';

// Internal imports
import {
  stableEnvKey,
  getCodexInstance,
  activeThreads,
  activeTurnAbortControllers,
} from './codex/instance.js';
import { normalizeReasoningEffort, tryParseJsonObject, buildCodexInput } from './codex/utils.js';
import { itemToEvents } from './codex/events.js';

/**
 * Execute a Codex query with streaming and yield events back
 */
export async function* executeCodexQuery(
  request: CodexQueryRequest,
  store: SessionStore,
): AsyncGenerator<BridgeEvent> {
  const startTime = Date.now();
  let totalInputTokens = 0;
  let totalOutputTokens = 0;
  let totalCacheReadTokens = 0;
  let success = true;
  let lastAgentMessage: string | null = null;

  const envKey = stableEnvKey(request.env);
  const codex = getCodexInstance(request.env);

  const threadOptions: ThreadOptions = {
    workingDirectory: request.cwd,
    sandboxMode: request.sandbox,
    approvalPolicy: request.approvalPolicy ?? 'never',
    skipGitRepoCheck: request.skipGitRepoCheck,
    model: request.model,
    modelReasoningEffort: normalizeReasoningEffort(request.effort),
  };

  let thread: ReturnType<typeof codex.startThread>;
  let threadId: string | null = request.threadId ?? null;
  const fallbackThreadId = threadId ?? uuidv4();

  const abortController = new AbortController();
  activeTurnAbortControllers.set(fallbackThreadId, abortController);

  if (threadId && activeThreads.has(threadId)) {
    const cached = activeThreads.get(threadId)!;
    if (cached.envKey === envKey) {
      thread = cached.thread;
    } else {
      thread = codex.resumeThread(threadId, threadOptions);
      activeThreads.set(threadId, { envKey, thread });
    }
  } else if (threadId) {
    thread = codex.resumeThread(threadId, threadOptions);
    activeThreads.set(threadId, { envKey, thread });
  } else {
    thread = codex.startThread(threadOptions);
  }

  const MAX_RETRIES = 15;
  const BASE_RETRY_DELAY_MS = 1000;

  // Calculate retry delay with exponential backoff + jitter (matches Rust side)
  const getRetryDelay = (attempt: number): number => {
    const baseDelay = Math.min(BASE_RETRY_DELAY_MS * Math.pow(2, attempt - 1), 30000);
    const jitter = Math.random() * Math.min(baseDelay * 0.1, 1000);
    return baseDelay + jitter;
  };

  try {
    if (threadId) {
      yield {
        type: 'session.start',
        sessionId: threadId,
        timestamp: Date.now(),
        model: 'codex',
        tools: ['Bash', 'Read', 'Write', 'Edit', 'Search', 'WebSearch'],
      };
    }

    const turnOptions: TurnOptions = { signal: abortController.signal };
    if (request.outputSchema) {
      turnOptions.outputSchema = request.outputSchema;
    }

    const { input, cleanup } = await buildCodexInput(request);
    let retryCount = 0;
    let runSuccess = false;

    while (!runSuccess && retryCount <= MAX_RETRIES) {
      try {
        const { events } = await thread.runStreamed(input, turnOptions);

        for await (const event of events) {
        switch (event.type) {
          case 'thread.started': {
            const startedId = event.thread_id;
            threadId = startedId;
            activeThreads.set(startedId, { envKey, thread });

            if (activeTurnAbortControllers.has(fallbackThreadId)) {
              const ctrl = activeTurnAbortControllers.get(fallbackThreadId)!;
              activeTurnAbortControllers.delete(fallbackThreadId);
              activeTurnAbortControllers.set(startedId, ctrl);
            }

            yield {
              type: 'session.start',
              sessionId: startedId,
              timestamp: Date.now(),
              model: 'codex',
              tools: ['Bash', 'Read', 'Write', 'Edit', 'Search', 'WebSearch'],
            };
            break;
          }

          case 'item.started':
          case 'item.updated':
          case 'item.completed': {
            const currentThreadId = threadId ?? fallbackThreadId;
            const eventType = event.type.split('.')[1] as 'started' | 'updated' | 'completed';

            if (event.type === 'item.completed' && event.item.type === 'agent_message') {
              lastAgentMessage = event.item.text;
            }

            for (const bridgeEvent of itemToEvents(event.item, currentThreadId, eventType)) {
              yield bridgeEvent;
            }
            break;
          }

          case 'turn.completed':
            totalInputTokens += event.usage.input_tokens;
            totalCacheReadTokens += event.usage.cached_input_tokens;
            totalOutputTokens += event.usage.output_tokens;
            break;

          case 'turn.failed': {
            // Check if this is a retriable error (connection issues, rate limits)
            const msg = event.error.message.toLowerCase();
            const isRetriable = msg.includes('connection') ||
                               msg.includes('network') ||
                               msg.includes('timeout') ||
                               msg.includes('reconnect') ||
                               msg.includes('reset') ||
                               msg.includes('econnreset') ||
                               msg.includes('epipe') ||
                               msg.includes('socket') ||
                               event.error.message.includes('429') ||
                               msg.includes('rate limit') ||
                               msg.includes('too many requests');

            if (isRetriable && retryCount < MAX_RETRIES) {
              // Throw to trigger retry
              throw new Error(event.error.message);
            }

            success = false;
            yield {
              type: 'error',
              sessionId: threadId ?? 'unknown',
              timestamp: Date.now(),
              message: event.error.message,
            };
            break;
          }

          case 'error': {
            // Check if this is a retriable error
            const errMsg = event.message.toLowerCase();
            const isRetriableError = errMsg.includes('connection') ||
                                    errMsg.includes('network') ||
                                    errMsg.includes('timeout') ||
                                    errMsg.includes('reconnect') ||
                                    errMsg.includes('reset') ||
                                    errMsg.includes('econnreset') ||
                                    errMsg.includes('epipe') ||
                                    errMsg.includes('socket') ||
                                    event.message.includes('429') ||
                                    errMsg.includes('rate limit') ||
                                    errMsg.includes('too many requests');

            if (isRetriableError && retryCount < MAX_RETRIES) {
              throw new Error(event.message);
            }

            success = false;
            yield {
              type: 'error',
              sessionId: threadId ?? 'unknown',
              timestamp: Date.now(),
              message: event.message,
            };
            break;
          }
        }
        }
        // If we completed the event loop without throwing, mark as success
        runSuccess = true;
      } catch (turnError) {
        // Connection dropped or turn failed - retry if we haven't exceeded max retries
        retryCount++;
        if (retryCount > MAX_RETRIES) {
          throw turnError; // Re-throw to outer catch
        }
        const errorMsg = turnError instanceof Error ? turnError.message : String(turnError);
        const delay = getRetryDelay(retryCount);
        yield {
          type: 'error',
          sessionId: threadId ?? fallbackThreadId,
          timestamp: Date.now(),
          message: `Connection lost, retrying in ${Math.round(delay / 1000)}s (${retryCount}/${MAX_RETRIES})... ${errorMsg}`,
        };
        await new Promise(r => setTimeout(r, delay));
      }
    } // end while retry loop

    // Cleanup after successful completion or max retries
    await cleanup();

    const structuredResult = request.outputSchema && lastAgentMessage
      ? tryParseJsonObject(lastAgentMessage)
      : null;

    yield {
      type: 'session.complete',
      sessionId: threadId ?? 'unknown',
      timestamp: Date.now(),
      success,
      ...(structuredResult ? { result: structuredResult } : {}),
      usage: {
        inputTokens: Math.max(0, totalInputTokens - totalCacheReadTokens),
        outputTokens: totalOutputTokens,
        cacheReadTokens: totalCacheReadTokens,
      },
      durationMs: Date.now() - startTime,
    };

    const finalThreadId = threadId ?? fallbackThreadId;
    const existingSession = store.get(finalThreadId);
    store.upsert({
      id: finalThreadId,
      type: 'codex',
      createdAt: existingSession?.createdAt || startTime,
      lastActiveAt: Date.now(),
      cwd: request.cwd,
      turnCount: (existingSession?.turnCount || 0) + 1,
      totalTokens: (existingSession?.totalTokens || 0) + totalInputTokens + totalOutputTokens,
      totalCostUsd: 0,
    });

  } catch (error) {
    success = false;
    const finalThreadId = threadId ?? fallbackThreadId;
    yield {
      type: 'error',
      sessionId: finalThreadId,
      timestamp: Date.now(),
      message: error instanceof Error ? error.message : String(error),
    };

    yield {
      type: 'session.complete',
      sessionId: finalThreadId,
      timestamp: Date.now(),
      success: false,
      durationMs: Date.now() - startTime,
    };
  }

  const finalThreadId = threadId ?? fallbackThreadId;
  activeTurnAbortControllers.delete(finalThreadId);
  activeThreads.delete(finalThreadId);

  if (request.threadId && request.threadId !== finalThreadId) {
    activeThreads.delete(request.threadId);
  }
}
