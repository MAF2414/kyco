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
