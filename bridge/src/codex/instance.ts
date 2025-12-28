/**
 * Codex instance and thread management
 */

import { Codex } from '@openai/codex-sdk';

/** Cache of Codex instances */
export const codexInstances = new Map<string, InstanceType<typeof Codex>>();

/** Active threads for fast resume within this process */
export const activeThreads = new Map<
  string,
  { envKey: string; thread: ReturnType<InstanceType<typeof Codex>['startThread']> }
>();

/** Abort controllers for running turns (for interrupt support) */
export const activeTurnAbortControllers = new Map<string, AbortController>();

export function stableEnvKey(overrides?: Record<string, string>): string {
  if (!overrides || Object.keys(overrides).length === 0) return 'default';
  return Object.entries(overrides)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k}=${v}`)
    .join('\n');
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

export function getCodexInstance(envOverride?: Record<string, string>): InstanceType<typeof Codex> {
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

/** List available Codex threads */
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

/** Clear a thread from the cache */
export function clearCodexThread(threadId: string): boolean {
  return activeThreads.delete(threadId);
}
