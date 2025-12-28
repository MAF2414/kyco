/**
 * Session Store - SQLite-backed session metadata storage
 *
 * Persists session metadata to a SQLite database on disk (or in-memory when
 * `:memory:` is used). Automatically migrates legacy JSON storage when found.
 */

import Database from 'better-sqlite3';
import { existsSync, mkdirSync } from 'fs';
import path from 'path';
import { parseSession, prepareLegacyMigration, readLegacyJsonSessions } from './store-helpers.js';
import type { StoredSession } from './types.js';

type SessionType = StoredSession['type'];

export class SessionStore {
  private readonly dbPath: string;
  private readonly inMemory: boolean;
  private readonly db: Database.Database;

  private upsertStmt!: Database.Statement;
  private getStmt!: Database.Statement;
  private listAllStmt!: Database.Statement;
  private listByTypeStmt!: Database.Statement;
  private deleteStmt!: Database.Statement;
  private pruneStmt!: Database.Statement;
  private hasAnyStmt!: Database.Statement;

  constructor(dbPath?: string) {
    const requestedPath = (dbPath ?? ':memory:').trim();
    const initialPath = requestedPath.length === 0 ? ':memory:' : requestedPath;

    const { effectiveDbPath, legacySessions, legacyBackupPath } = prepareLegacyMigration(initialPath);
    this.dbPath = effectiveDbPath;
    this.inMemory = effectiveDbPath === ':memory:';

    this.db = this.openDatabase(this.dbPath);
    this.initSchema();
    this.prepareStatements();

    if (legacySessions) {
      const imported = this.importSessions(legacySessions);
      console.log(
        `Migrated ${imported} sessions from legacy JSON (${legacyBackupPath ?? initialPath}) into SQLite (${this.dbPath})`,
      );
    } else {
      this.migrateSiblingLegacyJsonIfNeeded();
    }
  }

  private openDatabase(dbPathToUse: string): Database.Database {
    try {
      if (dbPathToUse !== ':memory:') {
        const dir = path.dirname(dbPathToUse);
        if (dir && dir !== '.' && !existsSync(dir)) {
          mkdirSync(dir, { recursive: true });
        }
      }

      const db = new Database(dbPathToUse, { timeout: 5000 });
      if (dbPathToUse !== ':memory:') {
        db.pragma('journal_mode = WAL');
      }
      db.pragma('foreign_keys = ON');
      return db;
    } catch (error) {
      console.warn(`Failed to open SQLite session DB at "${dbPathToUse}", falling back to in-memory DB: ${String(error)}`);
      const db = new Database(':memory:');
      db.pragma('foreign_keys = ON');
      return db;
    }
  }

  private initSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        type TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        last_active INTEGER NOT NULL,
        data TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_sessions_last_active ON sessions(last_active);
      CREATE INDEX IF NOT EXISTS idx_sessions_type_last_active ON sessions(type, last_active DESC);
    `);
  }

  private prepareStatements(): void {
    this.upsertStmt = this.db.prepare(`
      INSERT INTO sessions (id, type, created_at, last_active, data)
      VALUES (@id, @type, @created_at, @last_active, @data)
      ON CONFLICT(id) DO UPDATE SET
        type = excluded.type,
        created_at = excluded.created_at,
        last_active = excluded.last_active,
        data = excluded.data
    `);
    this.getStmt = this.db.prepare(`SELECT data FROM sessions WHERE id = ?`);
    this.listAllStmt = this.db.prepare(`SELECT data FROM sessions ORDER BY last_active DESC LIMIT @limit`);
    this.listByTypeStmt = this.db.prepare(
      `SELECT data FROM sessions WHERE type = @type ORDER BY last_active DESC LIMIT @limit`,
    );
    this.deleteStmt = this.db.prepare(`DELETE FROM sessions WHERE id = ?`);
    this.pruneStmt = this.db.prepare(`DELETE FROM sessions WHERE last_active < ?`);
    this.hasAnyStmt = this.db.prepare(`SELECT 1 FROM sessions LIMIT 1`);
  }

  private importSessions(sessions: StoredSession[]): number {
    const txn = this.db.transaction((sessionsToImport: StoredSession[]) => {
      for (const session of sessionsToImport) {
        this.upsertStmt.run({
          id: session.id,
          type: session.type,
          created_at: session.createdAt,
          last_active: session.lastActiveAt,
          data: JSON.stringify(session),
        });
      }
    });

    try {
      txn(sessions);
      return sessions.length;
    } catch (error) {
      console.warn(`Failed to import legacy sessions: ${String(error)}`);
      return 0;
    }
  }

  private migrateSiblingLegacyJsonIfNeeded(): void {
    if (this.inMemory) return;

    try {
      const hasAny = this.hasAnyStmt.get();
      if (hasAny) return;
    } catch {
      return;
    }

    const legacyJsonPath = path.join(path.dirname(this.dbPath), 'kyco-sessions.json');
    if (!existsSync(legacyJsonPath)) return;
    if (path.resolve(legacyJsonPath) === path.resolve(this.dbPath)) return;

    const sessions = readLegacyJsonSessions(legacyJsonPath);
    if (!sessions || sessions.length === 0) return;

    const imported = this.importSessions(sessions);
    if (imported > 0) {
      console.log(`Migrated ${imported} sessions from ${legacyJsonPath} into SQLite (${this.dbPath})`);
    }
  }

  /** Create or update a session */
  upsert(session: StoredSession): void {
    try {
      this.upsertStmt.run({
        id: session.id,
        type: session.type,
        created_at: session.createdAt,
        last_active: session.lastActiveAt,
        data: JSON.stringify(session),
      });
    } catch (error) {
      console.warn(`Failed to upsert session "${session.id}": ${String(error)}`);
    }
  }

  /** Get a session by ID */
  get(id: string): StoredSession | null {
    try {
      const row = this.getStmt.get(id) as { data?: unknown } | undefined;
      if (!row) return null;
      return parseSession(row.data);
    } catch (error) {
      console.warn(`Failed to get session "${id}": ${String(error)}`);
      return null;
    }
  }

  /** List all sessions, optionally filtered by type */
  list(type?: SessionType, limit = 100): StoredSession[] {
    try {
      const rows = (type ? this.listByTypeStmt.all({ type, limit }) : this.listAllStmt.all({ limit })) as Array<{
        data?: unknown;
      }>;

      const sessions: StoredSession[] = [];
      for (const row of rows) {
        const session = parseSession(row.data);
        if (session) sessions.push(session);
      }
      return sessions;
    } catch (error) {
      console.warn(`Failed to list sessions: ${String(error)}`);
      return [];
    }
  }

  /** Delete a session */
  delete(id: string): boolean {
    try {
      const result = this.deleteStmt.run(id);
      return result.changes > 0;
    } catch (error) {
      console.warn(`Failed to delete session "${id}": ${String(error)}`);
      return false;
    }
  }

  /** Delete old sessions (older than given days) */
  prune(olderThanDays: number): number {
    const cutoff = Date.now() - olderThanDays * 24 * 60 * 60 * 1000;

    try {
      const result = this.pruneStmt.run(cutoff);
      return result.changes;
    } catch (error) {
      console.warn(`Failed to prune sessions: ${String(error)}`);
      return 0;
    }
  }

  /** Close the store */
  close(): void {
    try {
      this.db.close();
    } catch {
      // ignore
    }
  }
}
