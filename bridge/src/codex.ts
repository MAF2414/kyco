/**
 * Codex SDK Integration
 *
 * Wraps the OpenAI Codex SDK to provide a streaming interface for KYCO.
 * Based on the samples in docs/codexSamples/
 */

import { Codex } from '@openai/codex-sdk';
import type { ModelReasoningEffort, ThreadItem, ThreadOptions, TurnOptions } from '@openai/codex-sdk';
import { v4 as uuidv4 } from 'uuid';
import type { CodexQueryRequest, BridgeEvent } from './types.js';
import { SessionStore } from './store.js';
import os from 'os';
import path from 'path';
import { promises as fs } from 'fs';

function normalizeReasoningEffort(
  effort: CodexQueryRequest['effort'],
): ModelReasoningEffort {
  if (effort === 'none') return undefined as unknown as ModelReasoningEffort;
  if (!effort) return 'xhigh' as ModelReasoningEffort; // Default to maximum effort
  // Pass through all effort levels including 'xhigh' - the SDK supports it at runtime
  return effort as ModelReasoningEffort;
}

function tryParseJsonObject(text: string): Record<string, unknown> | null {
  const trimmed = text.trim();

  // Strip ```json fences if present.
  const fenced = trimmed.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  const candidate = fenced ? fenced[1].trim() : trimmed;

  const start = candidate.indexOf('{');
  const end = candidate.lastIndexOf('}');
  const jsonCandidate = start >= 0 && end > start ? candidate.slice(start, end + 1) : candidate;

  try {
    const parsed: unknown = JSON.parse(jsonCandidate);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // ignore
  }

  return null;
}

/** Cache of Codex instances */
const codexInstances = new Map<string, InstanceType<typeof Codex>>();

/** Active threads for fast resume within this process */
const activeThreads = new Map<
  string,
  { envKey: string; thread: ReturnType<InstanceType<typeof Codex>['startThread']> }
>();

/** Abort controllers for running turns (for interrupt support) */
const activeTurnAbortControllers = new Map<string, AbortController>();

type CodexUserInput =
  | { type: 'text'; text: string }
  | { type: 'local_image'; path: string };

function normalizeImageBase64Data(data: string): { data: string; mediaType?: string } {
  const trimmed = data.trim();
  const match = /^data:([^;]+);base64,(.*)$/s.exec(trimmed);
  if (match) {
    return { mediaType: match[1], data: match[2]?.trim() ?? '' };
  }
  return { data: trimmed };
}

function extensionForMediaType(mediaType: string): string {
  switch (mediaType.toLowerCase()) {
    case 'image/jpeg':
    case 'image/jpg':
      return 'jpg';
    case 'image/webp':
      return 'webp';
    case 'image/gif':
      return 'gif';
    case 'image/png':
    default:
      return 'png';
  }
}

async function buildCodexInput(
  request: CodexQueryRequest,
): Promise<{ input: string | CodexUserInput[]; cleanup: () => Promise<void> }> {
  if (!request.images || request.images.length === 0) {
    return { input: request.prompt, cleanup: async () => {} };
  }

  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'kyco-codex-images-'));
  const files: string[] = [];

  for (const [index, image] of request.images.entries()) {
    const normalized = normalizeImageBase64Data(image.data);
    const mediaType = image.mediaType ?? normalized.mediaType ?? 'image/png';
    const ext = extensionForMediaType(mediaType);
    const filePath = path.join(dir, `image-${index}.${ext}`);

    const bytes = Buffer.from(normalized.data, 'base64');
    await fs.writeFile(filePath, bytes);
    files.push(filePath);
  }

  const cleanup = async () => {
    try {
      await fs.rm(dir, { recursive: true, force: true });
    } catch {
      // ignore
    }
  };

  return {
    input: [
      { type: 'text', text: request.prompt },
      ...files.map((p) => ({ type: 'local_image' as const, path: p })),
    ],
    cleanup,
  };
}

/**
 * Get or create a Codex instance
 */
function stableEnvKey(overrides?: Record<string, string>): string {
  if (!overrides || Object.keys(overrides).length === 0) return 'default';
  return Object.entries(overrides)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k}=${v}`)
    .join('\n');
}

function mergeEnv(overrides?: Record<string, string>): Record<string, string> {
  const base: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (typeof value === 'string') {
      base[key] = value;
    }
  }
  return { ...base, ...(overrides ?? {}) };
}

function getCodexInstance(envOverride?: Record<string, string>): InstanceType<typeof Codex> {
  const envKey = stableEnvKey(envOverride);
  let instance = codexInstances.get(envKey);
  if (!instance) {
    const mergedEnv = mergeEnv(envOverride);
    // Use CODEX_EXECUTABLE env var if set, otherwise use default 'codex'
    const codexPath = mergedEnv.CODEX_EXECUTABLE;
    instance = codexPath
      ? new Codex({ codexPathOverride: codexPath, env: mergedEnv })
      : new Codex({ env: mergedEnv });
    codexInstances.set(envKey, instance);
  }
  return instance;
}

/**
 * Convert a Codex ThreadItem to KYCO BridgeEvents
 */
function* itemToEvents(
  item: ThreadItem,
  sessionId: string,
  eventType: 'started' | 'updated' | 'completed'
): Generator<BridgeEvent> {
  const timestamp = Date.now();

  switch (item.type) {
    case 'agent_message':
      yield {
        type: 'text',
        sessionId,
        timestamp,
        content: item.text,
        partial: eventType !== 'completed',
      };
      break;

    case 'reasoning':
      // Emit reasoning as a special text event
      yield {
        type: 'text',
        sessionId,
        timestamp,
        content: `[Reasoning] ${item.text}`,
        partial: eventType !== 'completed',
      };
      break;

    case 'command_execution': {
      const toolUseId = `cmd-${item.id}`;
      if (eventType === 'started') {
        yield {
          type: 'tool.use',
          sessionId,
          timestamp,
          toolName: 'Bash',
          toolInput: { command: item.command },
          toolUseId,
        };
      } else if (eventType === 'completed') {
        yield {
          type: 'tool.result',
          sessionId,
          timestamp,
          toolUseId,
          success: item.exit_code === 0,
          output: `Exit code: ${item.exit_code ?? 'unknown'}`,
        };
      }
      break;
    }

    case 'file_change':
      if (eventType === 'completed') {
        for (const change of item.changes) {
          yield {
            type: 'tool.result',
            sessionId,
            timestamp,
            toolUseId: `file-${item.id}`,
            success: item.status === 'completed',
            output: `File ${change.kind}: ${change.path}`,
            filesChanged: [change.path],
          };
        }
      }
      break;

    case 'todo_list':
      // Emit todo list as text
      const todoText = item.items
        .map(todo => `[${todo.completed ? 'x' : ' '}] ${todo.text}`)
        .join('\n');
      yield {
        type: 'text',
        sessionId,
        timestamp,
        content: `Todo List:\n${todoText}`,
        partial: false,
      };
      break;

    case 'mcp_tool_call': {
      // Format tool name as mcp:<server>:<tool> (e.g., "mcp:github:create_issue")
      const mcpToolName = `mcp:${item.server}:${item.tool}`;
      const mcpToolUseId = `mcp-${item.id}`;

      if (eventType === 'started') {
        yield {
          type: 'tool.use',
          sessionId,
          timestamp,
          toolName: mcpToolName,
          toolInput: (item.arguments as Record<string, unknown>) ?? {},
          toolUseId: mcpToolUseId,
        };
      } else if (eventType === 'completed') {
        // Determine success based on status and presence of error
        const success = item.status === 'completed' && !item.error;

        // Build output string from result or error
        let output: string;
        if (item.error) {
          output = `Error: ${item.error.message}`;
        } else if (item.result) {
          // Extract text content from MCP result blocks
          const textParts = item.result.content
            .filter((block): block is { type: 'text'; text: string } => block.type === 'text')
            .map(block => block.text);
          output = textParts.length > 0
            ? textParts.join('\n')
            : JSON.stringify(item.result.structured_content ?? item.result.content);
        } else {
          output = 'MCP tool call completed';
        }

        yield {
          type: 'tool.result',
          sessionId,
          timestamp,
          toolUseId: mcpToolUseId,
          success,
          output,
        };
      }
      break;
    }

    case 'web_search': {
      // Emit web search as a tool use/result pair to show the user what was searched
      const webSearchToolUseId = `websearch-${item.id}`;
      if (eventType === 'started') {
        yield {
          type: 'tool.use',
          sessionId,
          timestamp,
          toolName: 'WebSearch',
          toolInput: { query: item.query },
          toolUseId: webSearchToolUseId,
        };
      } else if (eventType === 'completed') {
        yield {
          type: 'tool.result',
          sessionId,
          timestamp,
          toolUseId: webSearchToolUseId,
          success: true,
          output: `Web search completed for: "${item.query}"`,
        };
      }
      break;
    }

    default: {
      // Handle plan_update and other future item types not yet in SDK types
      // The Codex SDK documentation mentions "plan updates" as an item type,
      // but the TypeScript types may lag behind runtime behavior
      const unknownItem = item as { type: string; id: string; [key: string]: unknown };

      if (unknownItem.type === 'plan_update') {
        // Emit plan update as a special text event, similar to reasoning
        // Include old vs new plan when available for visibility into plan changes
        const planItem = unknownItem as {
          type: 'plan_update';
          id: string;
          old_plan?: string;
          new_plan?: string;
          plan?: string;
        };

        let planContent: string;
        if (planItem.old_plan && planItem.new_plan) {
          planContent = `Previous plan:\n${planItem.old_plan}\n\nUpdated plan:\n${planItem.new_plan}`;
        } else if (planItem.new_plan) {
          planContent = planItem.new_plan;
        } else if (planItem.plan) {
          planContent = planItem.plan;
        } else {
          planContent = 'Plan updated';
        }

        yield {
          type: 'text',
          sessionId,
          timestamp,
          content: `[Plan Update] ${planContent}`,
          partial: eventType !== 'completed',
        };
      }
      // Silently ignore other unknown item types
      break;
    }
  }
}

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
  let success = true;
  let lastAgentMessage: string | null = null;

  const envKey = stableEnvKey(request.env);
  const codex = getCodexInstance(request.env);

  const threadOptions: ThreadOptions = {
    workingDirectory: request.cwd,
    sandboxMode: request.sandbox,
    // Use configurable approval policy (default: 'never' for backward compatibility)
    approvalPolicy: request.approvalPolicy ?? 'never',
    // Skip git repo requirement when explicitly requested (for temp directories, non-git projects)
    skipGitRepoCheck: request.skipGitRepoCheck,
    // Model and reasoning effort configuration
    model: request.model,
    modelReasoningEffort: normalizeReasoningEffort(request.effort),
  };

  // Get or create thread
  let thread: ReturnType<InstanceType<typeof Codex>['startThread']>;
  let threadId: string | null = request.threadId ?? null;
  const fallbackThreadId = threadId ?? uuidv4();

  // Abort support (can be triggered via /codex/interrupt)
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

  try {
    // Emit session start early when resuming.
    if (threadId) {
      yield {
        type: 'session.start',
        sessionId: threadId,
        timestamp: Date.now(),
        model: 'codex',
        tools: ['Bash', 'Read', 'Write', 'Edit', 'Search', 'WebSearch'],
      };
    }

    // Run with streaming
    const turnOptions: TurnOptions = {
      signal: abortController.signal,
    };
    if (request.outputSchema) {
      turnOptions.outputSchema = request.outputSchema;
    }

    const { input, cleanup } = await buildCodexInput(request);
    try {
      const { events } = await thread.runStreamed(input, turnOptions);

      // Process streamed events
      for await (const event of events) {
        switch (event.type) {
          case 'thread.started': {
            const startedId = event.thread_id;
            threadId = startedId;
            activeThreads.set(startedId, { envKey, thread });

            // Move abort controller mapping to the real thread id.
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
            totalOutputTokens += event.usage.output_tokens;
            // Don't emit complete yet - might be multi-turn
            break;

          case 'turn.failed':
            success = false;
            yield {
              type: 'error',
              sessionId: threadId ?? 'unknown',
              timestamp: Date.now(),
              message: event.error.message,
            };
            break;

          case 'error':
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
    } finally {
      await cleanup();
    }

    // Session complete
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
        inputTokens: totalInputTokens,
        outputTokens: totalOutputTokens,
      },
      durationMs: Date.now() - startTime,
    };

    // Update session store
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
      totalCostUsd: 0, // Codex doesn't expose cost yet
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

  // Cleanup abort controller and thread cache
  // Removing from activeThreads prevents memory leaks - threads can be resumed
  // via codex.resumeThread() which will reload state from disk
  const finalThreadId = threadId ?? fallbackThreadId;
  activeTurnAbortControllers.delete(finalThreadId);
  activeThreads.delete(finalThreadId);

  // Also cleanup the original request threadId if it differs (edge case: ID changed during session)
  if (request.threadId && request.threadId !== finalThreadId) {
    activeThreads.delete(request.threadId);
  }
}

/**
 * Execute a Codex query with structured output
 */
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

/**
 * List available Codex threads
 */
export function listCodexThreads(): string[] {
  return Array.from(activeThreads.keys());
}

export function interruptCodexThread(threadId: string): boolean {
  const controller = activeTurnAbortControllers.get(threadId);
  if (!controller) {
    return false;
  }
  controller.abort();
  return true;
}

/**
 * Clear a thread from the cache
 */
export function clearCodexThread(threadId: string): boolean {
  return activeThreads.delete(threadId);
}
