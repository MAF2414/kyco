import * as vscode from 'vscode';
import * as path from 'path';
import type { KycoHttpConfig } from './types';
import { KYCO_DEFAULT_PORT } from './types';

// In-memory cache keyed by workspace path; refreshed when `.kyco/config.toml` changes.
const httpConfigCache = new Map<string, KycoHttpConfig>();

export async function getKycoHttpConfig(workspace: string): Promise<KycoHttpConfig> {
    if (!workspace) {
        return { port: KYCO_DEFAULT_PORT };
    }

    const configUri = vscode.Uri.file(path.join(workspace, '.kyco', 'config.toml'));

    try {
        const stat = await vscode.workspace.fs.stat(configUri);
        const cached = httpConfigCache.get(workspace);
        if (cached && cached.mtime === stat.mtime) {
            return cached;
        }

        const bytes = await vscode.workspace.fs.readFile(configUri);
        const text = Buffer.from(bytes).toString('utf8');
        const token = extractHttpTokenFromToml(text);
        const port = extractHttpPortFromToml(text) ?? KYCO_DEFAULT_PORT;
        const cfg: KycoHttpConfig = { port, token, mtime: stat.mtime };
        httpConfigCache.set(workspace, cfg);
        return cfg;
    } catch {
        // Don't cache failures/misses: users may start `kyco gui` after the first send attempt.
        httpConfigCache.delete(workspace);
        return { port: KYCO_DEFAULT_PORT };
    }
}

function extractHttpTokenFromToml(tomlText: string): string | undefined {
    const lines = tomlText.split(/\r?\n/);
    let inSettingsGui = false;

    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || line.startsWith('#')) {
            continue;
        }

        // Table headers
        if (line.startsWith('[') && line.endsWith(']')) {
            inSettingsGui = line === '[settings.gui]';
            continue;
        }

        if (!inSettingsGui) {
            continue;
        }

        // http_token = "..."
        const mDouble = line.match(/^http_token\s*=\s*"([^"]*)"\s*(?:#.*)?$/);
        if (mDouble) {
            return mDouble[1] || undefined;
        }

        // http_token = '...'
        const mSingle = line.match(/^http_token\s*=\s*'([^']*)'\s*(?:#.*)?$/);
        if (mSingle) {
            return mSingle[1] || undefined;
        }
    }

    return undefined;
}

function extractHttpPortFromToml(tomlText: string): number | undefined {
    const lines = tomlText.split(/\r?\n/);
    let inSettingsGui = false;

    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || line.startsWith('#')) {
            continue;
        }

        if (line.startsWith('[') && line.endsWith(']')) {
            inSettingsGui = line === '[settings.gui]';
            continue;
        }

        if (!inSettingsGui) {
            continue;
        }

        const m = line.match(/^http_port\s*=\s*(\d+)\s*(?:#.*)?$/);
        if (m) {
            const parsed = Number(m[1]);
            if (Number.isFinite(parsed) && parsed > 0 && parsed < 65536) {
                return parsed;
            }
        }
    }

    return undefined;
}
