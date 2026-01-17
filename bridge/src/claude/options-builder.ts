/**
 * Build Claude query options from request
 */

import { query } from '@anthropic-ai/claude-agent-sdk';
import type {
  ClaudeQueryRequest,
  HookPreToolUseEvent,
  PermissionMode,
} from '../types.js';
import { mergeEnv } from './helpers.js';
import type { EventEmitter } from './types.js';
import { enforceToolUse, parseBugbountyPolicy } from '../policy/bugbounty.js';

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export function buildQueryOptions(
  request: ClaudeQueryRequest,
  sessionId: string,
  emitEvent: EventEmitter,
): Parameters<typeof query>[0]['options'] {
  const options: Parameters<typeof query>[0]['options'] = {
    cwd: request.cwd,
    permissionMode: (request.permissionMode || 'default') as PermissionMode,
  };

  options.settingSources = request.settingSources ?? ['user', 'project', 'local'];

  if (request.env) {
    options.env = mergeEnv(request.env);
  }

  if (options.permissionMode === 'bypassPermissions') {
    options.allowDangerouslySkipPermissions = true;
  }

  if (request.sessionId) {
    options.resume = request.sessionId;
    options.forkSession = request.forkSession ?? false;
  }

  if (request.sessionId && !request.forkSession) {
    options.continue = true;
  }

  if (request.allowedTools?.length) {
    options.allowedTools = request.allowedTools;
  }
  if (request.disallowedTools?.length) {
    options.disallowedTools = request.disallowedTools;
  }

  const bugbountyPolicy = parseBugbountyPolicy(request.env);
  const shouldEmitPreToolUse = request.hooks?.events?.includes('PreToolUse') ?? false;
  const shouldInstallPreToolUseHook = shouldEmitPreToolUse || !!bugbountyPolicy;

  if (shouldInstallPreToolUseHook) {
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
            const effectiveSessionId = sessionId || hookInput.session_id;
            const toolName = hookInput.tool_name;
            const toolInput = hookInput.tool_input as Record<string, unknown>;

            if (shouldEmitPreToolUse) {
              const event: HookPreToolUseEvent = {
                type: 'hook.pre_tool_use',
                sessionId: effectiveSessionId,
                timestamp: Date.now(),
                toolName,
                toolInput,
                toolUseId: hookInput.tool_use_id,
                transcriptPath: hookInput.transcript_path,
              };
              emitEvent(event);
            }

            const decision = enforceToolUse(effectiveSessionId, toolName, toolInput, bugbountyPolicy);
            if (!decision.allow) {
              return {
                continue: false,
                decision: 'block',
                reason: decision.reason,
                systemMessage: decision.systemMessage ?? decision.reason,
              };
            }
            if (decision.delayMs && decision.delayMs > 0) {
              await sleep(decision.delayMs);
            }
          } catch {
            // Hooks must never break execution.
          }
          return { continue: true };
        }],
      }],
    };
  }

  if (request.agents) {
    options.agents = request.agents;
  }
  if (request.mcpServers) {
    options.mcpServers = request.mcpServers;
  }
  if (request.plugins?.length) {
    options.plugins = request.plugins;
  }
  if (request.settingSources?.length) {
    options.settingSources = request.settingSources;
  }

  const promptMode = request.systemPromptMode ?? 'append';
  if (promptMode === 'replace' && request.systemPrompt) {
    options.systemPrompt = request.systemPrompt;
  } else {
    options.systemPrompt = request.systemPrompt
      ? { type: 'preset', preset: 'claude_code', append: request.systemPrompt }
      : { type: 'preset', preset: 'claude_code' };
  }

  if (request.maxTurns) {
    options.maxTurns = request.maxTurns;
  }
  if (request.outputSchema) {
    options.outputFormat = {
      type: 'json_schema',
      schema: request.outputSchema,
    };
  }

  options.maxThinkingTokens = request.maxThinkingTokens ?? 10000;

  if (request.model) {
    options.model = request.model;
  }

  return options;
}
