/**
 * Utility functions for Codex module
 */

import type { ModelReasoningEffort } from '@openai/codex-sdk';
import os from 'os';
import path from 'path';
import { promises as fs } from 'fs';
import type { CodexQueryRequest } from '../types.js';
import type { CodexUserInput } from './types.js';

export function normalizeReasoningEffort(
  effort: CodexQueryRequest['effort'],
): ModelReasoningEffort {
  if (effort === 'none') return undefined as unknown as ModelReasoningEffort;
  if (!effort) return 'xhigh' as ModelReasoningEffort; // Default to maximum effort
  // Pass through all effort levels including 'xhigh' - the SDK supports it at runtime
  return effort as ModelReasoningEffort;
}

export function tryParseJsonObject(text: string): Record<string, unknown> | null {
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

export function normalizeImageBase64Data(data: string): { data: string; mediaType?: string } {
  const trimmed = data.trim();
  const match = /^data:([^;]+);base64,(.*)$/s.exec(trimmed);
  if (match) {
    return { mediaType: match[1], data: match[2]?.trim() ?? '' };
  }
  return { data: trimmed };
}

export function extensionForMediaType(mediaType: string): string {
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

export async function buildCodexInput(
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
