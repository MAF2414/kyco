/**
 * Convert Codex ThreadItem to KYCO BridgeEvents
 */

import type { ThreadItem } from '@openai/codex-sdk';
import type { BridgeEvent } from '../types.js';

export function* itemToEvents(
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
      // Emit reasoning as a dedicated event type (not mixed with output text)
      yield {
        type: 'reasoning',
        sessionId,
        timestamp,
        content: item.text,
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
