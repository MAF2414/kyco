/**
 * Session Store Helpers - Utility functions for session storage
 *
 * Pure utility functions extracted from SessionStore for legacy migration
 * and session parsing.
 */

import { closeSync, existsSync, openSync, readFileSync, readSync, renameSync } from 'fs';
import type { StoredSession } from './types.js';

/** Parse JSON string to StoredSession, validating required fields */
export function parseSession(json: unknown): StoredSession | null {
  if (typeof json !== 'string') return null;
  try {
    const parsed = JSON.parse(json) as Partial<StoredSession> | null;
    if (!parsed || typeof parsed !== 'object') return null;
    if (
      typeof parsed.id !== 'string' ||
      (parsed.type !== 'claude' && parsed.type !== 'codex') ||
      typeof parsed.createdAt !== 'number' ||
      typeof parsed.lastActiveAt !== 'number' ||
      typeof parsed.cwd !== 'string' ||
      typeof parsed.turnCount !== 'number' ||
      typeof parsed.totalTokens !== 'number' ||
      typeof parsed.totalCostUsd !== 'number'
    ) {
      return null;
    }
    return parsed as StoredSession;
  } catch {
    return null;
  }
}

/** Read and parse legacy JSON sessions file */
export function readLegacyJsonSessions(filePath: string): StoredSession[] | null {
  try {
    const raw = readFileSync(filePath, 'utf-8');
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return null;
    return parsed as StoredSession[];
  } catch {
    return null;
  }
}

/** Get an available backup path for a file (adds .bak or .bak-timestamp) */
export function getAvailableBackupPath(originalPath: string): string {
  const base = `${originalPath}.bak`;
  if (!existsSync(base)) return base;
  return `${base}-${Date.now()}`;
}

/** Check if a file is a SQLite database by reading its magic header */
export function isSqliteFile(filePath: string): boolean {
  try {
    const fd = openSync(filePath, 'r');
    try {
      const buffer = Buffer.alloc(16);
      const bytesRead = readSync(fd, buffer, 0, 16, 0);
      if (bytesRead === 0) return true;
      if (bytesRead < 16) return false;
      return buffer.toString('utf8') === 'SQLite format 3\u0000';
    } finally {
      closeSync(fd);
    }
  } catch {
    return false;
  }
}

export interface LegacyMigrationPlan {
  effectiveDbPath: string;
  legacySessions: StoredSession[] | null;
  legacyBackupPath: string | null;
}

/**
 * Prepare legacy migration by checking if the requested path points to a legacy JSON file.
 * If so, renames it to a backup and returns the sessions to import.
 */
export function prepareLegacyMigration(requestedPath: string): LegacyMigrationPlan {
  if (requestedPath === ':memory:') {
    return { effectiveDbPath: ':memory:', legacySessions: null, legacyBackupPath: null };
  }

  if (!existsSync(requestedPath)) {
    return { effectiveDbPath: requestedPath, legacySessions: null, legacyBackupPath: null };
  }

  if (isSqliteFile(requestedPath)) {
    return { effectiveDbPath: requestedPath, legacySessions: null, legacyBackupPath: null };
  }

  const legacySessions = readLegacyJsonSessions(requestedPath);
  if (!legacySessions) {
    return { effectiveDbPath: requestedPath, legacySessions: null, legacyBackupPath: null };
  }

  const legacyBackupPath = getAvailableBackupPath(requestedPath);
  try {
    renameSync(requestedPath, legacyBackupPath);
  } catch (error) {
    console.warn(`Failed to back up legacy JSON at "${requestedPath}", using in-memory DB: ${String(error)}`);
    return { effectiveDbPath: ':memory:', legacySessions: null, legacyBackupPath: null };
  }

  return { effectiveDbPath: requestedPath, legacySessions, legacyBackupPath };
}
