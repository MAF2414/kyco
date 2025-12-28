/**
 * Claude Agent SDK Integration
 *
 * Wraps the Claude Agent SDK to provide a streaming interface for KYCO.
 */

import { query, type Query } from '@anthropic-ai/claude-agent-sdk';
import { v4 as uuidv4 } from 'uuid';
import type {
  ClaudeQueryRequest,
  BridgeEvent,
  HookPreToolUseEvent,
  ToolApprovalResponse,
  PermissionMode,
  ToolApprovalNeededEvent,
  HeartbeatEvent,
} from './types.js';
import { SessionStore } from './store.js';

/** Active queries that can be interrupted */
const activeQueries = new Map<string, Query>();

type ClaudeTextBlock = { type: 'text'; text: string };
type ClaudeImageBlock = {
  type: 'image';
  source: { type: 'base64'; media_type: string; data: string };
};
type ClaudeContentBlock = ClaudeTextBlock | ClaudeImageBlock;

type ClaudeMessageParam = {
  role: 'user';
  content: ClaudeContentBlock[];
};

type ClaudePromptMessage = {
  type: 'user';
  session_id: string;
  message: ClaudeMessageParam;
  parent_tool_use_id: null;
};

function normalizeImageBase64Data(data: string): { data: string; mediaType?: string } {
  const trimmed = data.trim();
  const match = /^data:([^;]+);base64,(.*)$/s.exec(trimmed);
  if (match) {
    return { mediaType: match[1], data: match[2]?.trim() ?? '' };
  }
  return { data: trimmed };
}

function buildPromptContentBlocks(request: ClaudeQueryRequest): ClaudeContentBlock[] {
  const blocks: ClaudeContentBlock[] = [];

  if (request.prompt.length > 0) {
    blocks.push({ type: 'text', text: request.prompt });
  }

  for (const image of request.images ?? []) {
    const normalized = normalizeImageBase64Data(image.data);
    const mediaType = image.mediaType ?? normalized.mediaType ?? 'image/png';
    blocks.push({
      type: 'image',
      source: {
        type: 'base64',
        media_type: mediaType,
        data: normalized.data,
      },
    });
  }

  if (blocks.length === 0) {
    blocks.push({ type: 'text', text: '' });
  }

  return blocks;
}

async function* buildClaudePrompt(request: ClaudeQueryRequest): AsyncIterable<ClaudePromptMessage> {
  yield {
    type: 'user',
    session_id: '',
    message: {
      role: 'user',
      content: buildPromptContentBlocks(request),
    },
    parent_tool_use_id: null,
  };
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

/**
 * Pending tool approval requests
 * Key: requestId, Value: Promise resolver and metadata
 */
const pendingApprovals = new Map<string, {
  resolve: (response: ToolApprovalResponse) => void;
  toolName: string;
  toolInput: Record<string, unknown>;
  sessionId: string;
  heartbeatInterval?: ReturnType<typeof setInterval>;
}>();

/**
 * Event emitter callback type for streaming events out
 */
type EventEmitter = (event: BridgeEvent) => void;

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
  // Use provided sessionId for resume, otherwise we'll get a new one from SDK
  let sessionId = request.sessionId || '';
  const startTime = Date.now();
  let totalInputTokens = 0;
  let totalOutputTokens = 0;
  let costUsd = 0;

  // Event queue for async generator
  const eventQueue: BridgeEvent[] = [];
  let resolveNext: (() => void) | null = null;
  let sessionCompleted = false; // Track if session.complete was sent

  // Set up event emitter
  const emitEvent = (event: BridgeEvent) => {
    eventQueue.push(event);
    if (resolveNext) {
      resolveNext();
      resolveNext = null;
    }
  };

  currentEventEmitter = emitEvent;

  // Track query for cleanup - must be accessible in catch block
  let q: Query | null = null;

  try {
    // Build the query options
    const options: Parameters<typeof query>[0]['options'] = {
      cwd: request.cwd,
      permissionMode: (request.permissionMode || 'default') as PermissionMode,
    };

    // Load Claude Code settings for parity with the CLI (incl. CLAUDE.md).
    // Note: SDK loads no settings unless `settingSources` is provided.
    options.settingSources = request.settingSources ?? ['user', 'project', 'local'];

    // Environment variables (merged with process.env)
    if (request.env) {
      options.env = mergeEnv(request.env);
    }

    if (options.permissionMode === 'bypassPermissions') {
      options.allowDangerouslySkipPermissions = true;
    }

    // Resume existing session if provided
    if (request.sessionId) {
      options.resume = request.sessionId;
      options.forkSession = request.forkSession ?? false;
    }

    // Continue flag for multi-turn in same session
    if (request.sessionId && !request.forkSession) {
      options.continue = true;
    }

    // Tool configuration
    if (request.allowedTools?.length) {
      options.allowedTools = request.allowedTools;
    }
    if (request.disallowedTools?.length) {
      options.disallowedTools = request.disallowedTools;
    }

    // Hook configuration (Claude SDK)
    // Phase 1: emit PreToolUse events only (non-blocking / no modifications).
    if (request.hooks?.events?.includes('PreToolUse')) {
      options.hooks = {
        PreToolUse: [{
          hooks: [async (input: unknown) => {
            try {
              const hookInput = input as {
                session_id: string;
                transcript_path: string;
                tool_name: string;
                tool_input: unknown;
                tool_use_id: string;
              };
              const event: HookPreToolUseEvent = {
                type: 'hook.pre_tool_use',
                sessionId: sessionId || hookInput.session_id,
                timestamp: Date.now(),
                toolName: hookInput.tool_name,
                toolInput: hookInput.tool_input as Record<string, unknown>,
                toolUseId: hookInput.tool_use_id,
                transcriptPath: hookInput.transcript_path,
              };
              emitEvent(event);
            } catch {
              // Hooks must never break execution.
            }
            return { continue: true };
          }],
        }],
      };
    }

    // Subagents (Claude SDK only)
    if (request.agents) {
      options.agents = request.agents;
    }

    // MCP server configuration (Claude SDK only)
    if (request.mcpServers) {
      options.mcpServers = request.mcpServers;
    }

    // Plugins (Claude SDK only)
    if (request.plugins?.length) {
      options.plugins = request.plugins;
    }

    // Load Claude Code settings sources (must include "project" for CLAUDE.md)
    if (request.settingSources?.length) {
      options.settingSources = request.settingSources;
    }

    // System prompt
    const promptMode = request.systemPromptMode ?? 'append';
    if (promptMode === 'replace' && request.systemPrompt) {
      options.systemPrompt = request.systemPrompt;
    } else {
      options.systemPrompt = request.systemPrompt
        ? { type: 'preset', preset: 'claude_code', append: request.systemPrompt }
        : { type: 'preset', preset: 'claude_code' };
    }

    // Max turns
    if (request.maxTurns) {
      options.maxTurns = request.maxTurns;
    }

    // Structured output
    if (request.outputSchema) {
      options.outputFormat = {
        type: 'json_schema',
        schema: request.outputSchema,
      };
    }

    // Extended thinking - enable by default with 10000 tokens
    // This allows Claude to "think" before responding, improving quality
    options.maxThinkingTokens = request.maxThinkingTokens ?? 10000;

    // Model selection
    if (request.model) {
      options.model = request.model;
    }

    // Add canUseTool callback for permission requests
    // This is only called when permissionMode is 'default' or 'acceptEdits'
    // and a tool needs explicit approval
    options.canUseTool = async (
      toolName: string,
      toolInput: Record<string, unknown>,
      callbackOptions?: { signal?: AbortSignal; suggestions?: unknown[] },
    ) => {
      const requestId = uuidv4();

      // Check if already aborted before starting
      if (callbackOptions?.signal?.aborted) {
        return {
          behavior: 'deny' as const,
          message: 'Session aborted',
        };
      }

      // Emit permission request event
      const approvalEvent: ToolApprovalNeededEvent = {
        type: 'tool.approval_needed',
        sessionId,
        timestamp: Date.now(),
        requestId,
        toolName,
        toolInput,
      };
      emitEvent(approvalEvent);

      // Create heartbeat emitter to keep HTTP connection alive while waiting
      const emitHeartbeat = () => {
        const heartbeatEvent: HeartbeatEvent = {
          type: 'heartbeat',
          sessionId,
          timestamp: Date.now(),
          pendingApprovalRequestId: requestId,
        };
        emitEvent(heartbeatEvent);
      };

      // Wait for KYCO to respond (with heartbeat to keep connection alive)
      // Also respect abort signal from the SDK
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
          behavior: 'allow' as const,
          updatedInput: response.modifiedInput || toolInput,
        };
      } else {
        return {
          behavior: 'deny' as const,
          message: response.reason || 'User denied permission',
        };
      }
    };

    // Start the query
    q = query({ prompt: buildClaudePrompt(request), options });

    // Process the stream in a background task
    const processStream = async () => {
      try {
        // q is guaranteed to be non-null here since it's set before processStream is called
        for await (const message of q!) {
          switch (message.type) {
            case 'system': {
              if (message.subtype === 'init') {
                // Capture the real session ID from the SDK
                sessionId = message.session_id;
                activeQueries.set(sessionId, q!);

                // Emit session start with real session ID
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
              const content = message.message.content;
              for (const block of content) {
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
              // Tool results come as user messages
              const content = message.message.content;
              for (const block of content) {
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
              sessionCompleted = true; // Mark that we sent session.complete
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

        // Fallback: If stream ended without sending session.complete, send one now
        // This ensures the client ALWAYS receives a session.complete event
        if (!sessionCompleted) {
          emitEvent({
            type: 'session.complete',
            sessionId,
            timestamp: Date.now(),
            success: true, // Stream ended normally, treat as success
            usage: {
              inputTokens: totalInputTokens,
              outputTokens: totalOutputTokens,
            },
            costUsd,
            durationMs: Date.now() - startTime,
          });
        }

        // Update session store
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

    // Start processing in background
    const streamPromise = processStream();

    // Yield events as they come in
    while (true) {
      if (eventQueue.length > 0) {
        const event = eventQueue.shift()!;
        yield event;

        // Stop if session completed
        if (event.type === 'session.complete') {
          break;
        }
      } else {
        // Wait for next event
        await new Promise<void>(resolve => {
          resolveNext = resolve;
          // Also check if the stream is done
          streamPromise.then(() => {
            if (resolveNext) {
              resolveNext();
              resolveNext = null;
            }
          });
        });

        // Check if we should exit
        if (eventQueue.length === 0) {
          // Stream finished without more events
          break;
        }
      }
    }

    // IMPORTANT: Close the query FIRST to unblock processStream, then wait for cleanup.
    // The processStream is iterating over `q` (for await of q), so it will block until
    // the SDK query closes. We must signal completion BEFORE waiting for streamPromise.
    if (q) {
      try {
        // First try interrupt() which is more forceful
        await q.interrupt();
      } catch {
        // If interrupt fails, try return()
        try {
          await q.return(undefined as never);
        } catch {
          // Ignore cleanup errors - session is already complete
        }
      }
    }

    // Wait for processStream with a timeout - don't block forever
    // Use Promise.race to ensure we exit even if cleanup hangs
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

    // Cleanup query on error - prevents memory leak from orphaned child processes
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
 * Wait for KYCO to respond to a tool approval request
 *
 * IMPORTANT: No timeout! The session stays paused until the user explicitly
 * decides to allow or deny. This is intentional for security - we never want
 * to auto-deny and cause Claude to retry in a loop, nor auto-allow dangerous
 * operations. The user MUST make the decision.
 *
 * We send periodic heartbeat events to keep the HTTP connection alive while waiting.
 * If an AbortSignal is provided and fires, we auto-deny to prevent hanging.
 */
function waitForApproval(
  requestId: string,
  toolName: string,
  toolInput: Record<string, unknown>,
  sessionId: string,
  emitHeartbeat?: () => void,
  signal?: AbortSignal,
): Promise<ToolApprovalResponse> {
  return new Promise((resolve) => {
    // Start heartbeat interval to keep HTTP connection alive
    const heartbeatInterval = emitHeartbeat
      ? setInterval(() => {
          emitHeartbeat();
        }, 15000) // Send heartbeat every 15 seconds
      : undefined;

    // Helper to cleanup and resolve
    const cleanup = (response: ToolApprovalResponse) => {
      if (heartbeatInterval) {
        clearInterval(heartbeatInterval);
      }
      pendingApprovals.delete(requestId);
      resolve(response);
    };

    // Listen for abort signal from SDK
    if (signal) {
      signal.addEventListener('abort', () => {
        cleanup({
          requestId,
          decision: 'deny',
          reason: 'Session aborted by SDK',
        });
      }, { once: true });
    }

    pendingApprovals.set(requestId, {
      resolve: cleanup,
      toolName,
      toolInput,
      sessionId,
      heartbeatInterval,
    });
  });
}

/**
 * Interrupt an active query
 */
export async function interruptClaudeQuery(sessionId: string): Promise<boolean> {
  const q = activeQueries.get(sessionId);
  if (q) {
    // Clean up any pending approvals for this session first
    // This prevents heartbeat intervals from running after interrupt
    for (const [requestId, approval] of pendingApprovals.entries()) {
      if (approval.sessionId === sessionId) {
        // Stop heartbeat interval
        if (approval.heartbeatInterval) {
          clearInterval(approval.heartbeatInterval);
        }
        // Resolve with interrupted status (so the Promise doesn't hang)
        approval.resolve({
          requestId,
          decision: 'deny',
          reason: 'Session interrupted by user',
        });
        pendingApprovals.delete(requestId);
      }
    }

    await q.interrupt();
    return true;
  }
  return false;
}

/**
 * Change permission mode for an active session.
 */
export async function setClaudePermissionMode(sessionId: string, mode: PermissionMode): Promise<boolean> {
  const q = activeQueries.get(sessionId);
  if (q) {
    await q.setPermissionMode(mode);
    return true;
  }
  return false;
}

/**
 * Resolve a pending tool approval request
 */
export function resolveToolApproval(response: ToolApprovalResponse): boolean {
  const pending = pendingApprovals.get(response.requestId);
  if (pending) {
    // Stop heartbeat interval (it's also stopped in the resolve callback, but be safe)
    if (pending.heartbeatInterval) {
      clearInterval(pending.heartbeatInterval);
    }
    pending.resolve(response);
    pendingApprovals.delete(response.requestId);
    return true;
  }
  return false;
}

/**
 * Get pending approval requests (for debugging/status)
 */
export function getPendingApprovals(): Array<{
  requestId: string;
  toolName: string;
  toolInput: Record<string, unknown>;
}> {
  return Array.from(pendingApprovals.entries()).map(([requestId, data]) => ({
    requestId,
    toolName: data.toolName,
    toolInput: data.toolInput,
  }));
}
