/**
 * Helper functions for Claude query building
 */

import type { ClaudeQueryRequest } from '../types.js';
import type { ClaudeContentBlock, ClaudePromptMessage } from './types.js';

export function normalizeImageBase64Data(data: string): { data: string; mediaType?: string } {
  const trimmed = data.trim();
  const match = /^data:([^;]+);base64,(.*)$/s.exec(trimmed);
  if (match) {
    return { mediaType: match[1], data: match[2]?.trim() ?? '' };
  }
  return { data: trimmed };
}

export function buildPromptContentBlocks(request: ClaudeQueryRequest): ClaudeContentBlock[] {
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

export async function* buildClaudePrompt(request: ClaudeQueryRequest): AsyncIterable<ClaudePromptMessage> {
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

export function mergeEnv(overrides?: Record<string, string>): Record<string, string> {
  const base: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (typeof value === 'string') {
      base[key] = value;
    }
  }
  return { ...base, ...(overrides ?? {}) };
}
