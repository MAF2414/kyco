/**
 * BaselineResolver - retrieves file content from different baseline sources
 * Supports Git commits/branches and local snapshots
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { exec } from 'child_process';
import { promisify } from 'util';
import type { DiffBaseline } from './types';

const execAsync = promisify(exec);

/**
 * Interface for baseline content resolution
 */
export interface IBaselineResolver {
    /**
     * Get file content from the baseline
     * @returns File content or null if file doesn't exist in baseline
     */
    getFileContent(filePath: string, baseline: DiffBaseline): Promise<string | null>;

    /**
     * Check if file exists in baseline
     */
    fileExistsInBaseline(filePath: string, baseline: DiffBaseline): Promise<boolean>;

    /**
     * List all files in the baseline
     */
    listFilesInBaseline(baseline: DiffBaseline): Promise<string[]>;
}

/**
 * Git-based baseline resolver
 * Retrieves content from Git commits and branches
 */
export class GitBaselineResolver implements IBaselineResolver {
    constructor(private workspaceRoot: string) {}

    async getFileContent(filePath: string, baseline: DiffBaseline): Promise<string | null> {
        if (baseline.type === 'working-tree') {
            // Working tree = current state, no diff possible
            return null;
        }

        if (baseline.type === 'snapshot') {
            // Delegate to SnapshotBaselineResolver
            return null;
        }

        try {
            const ref = this.getGitRef(baseline);
            const relativePath = this.toRelativePath(filePath);

            // git show <ref>:<path>
            const { stdout } = await execAsync(
                `git show "${ref}:${relativePath}"`,
                {
                    cwd: this.workspaceRoot,
                    maxBuffer: 10 * 1024 * 1024, // 10MB
                    encoding: 'utf-8',
                }
            );

            return stdout;
        } catch (error) {
            // File doesn't exist in baseline or git error
            return null;
        }
    }

    async fileExistsInBaseline(filePath: string, baseline: DiffBaseline): Promise<boolean> {
        if (baseline.type === 'working-tree' || baseline.type === 'snapshot') {
            return false;
        }

        try {
            const ref = this.getGitRef(baseline);
            const relativePath = this.toRelativePath(filePath);

            await execAsync(
                `git cat-file -e "${ref}:${relativePath}"`,
                { cwd: this.workspaceRoot }
            );

            return true;
        } catch {
            return false;
        }
    }

    async listFilesInBaseline(baseline: DiffBaseline): Promise<string[]> {
        if (baseline.type === 'working-tree' || baseline.type === 'snapshot') {
            return [];
        }

        try {
            const ref = this.getGitRef(baseline);

            // git ls-tree -r --name-only <ref>
            const { stdout } = await execAsync(
                `git ls-tree -r --name-only "${ref}"`,
                {
                    cwd: this.workspaceRoot,
                    maxBuffer: 10 * 1024 * 1024,
                }
            );

            return stdout.split('\n').filter(line => line.trim());
        } catch {
            return [];
        }
    }

    /**
     * Get the default baseline (HEAD~1 or main branch)
     */
    async getDefaultBaseline(): Promise<DiffBaseline> {
        try {
            // Try to get HEAD~1
            const { stdout } = await execAsync(
                'git rev-parse HEAD~1',
                { cwd: this.workspaceRoot }
            );

            return {
                type: 'commit',
                reference: stdout.trim(),
                timestamp: new Date(),
            };
        } catch {
            // Fall back to main/master
            try {
                const { stdout } = await execAsync(
                    'git rev-parse main',
                    { cwd: this.workspaceRoot }
                );

                return {
                    type: 'branch',
                    reference: 'main',
                    timestamp: new Date(),
                };
            } catch {
                // Try master
                const { stdout } = await execAsync(
                    'git rev-parse master',
                    { cwd: this.workspaceRoot }
                );

                return {
                    type: 'branch',
                    reference: 'master',
                    timestamp: new Date(),
                };
            }
        }
    }

    /**
     * Get list of recent commits for baseline selection
     */
    async getRecentCommits(count: number = 10): Promise<{ sha: string; message: string; date: Date }[]> {
        try {
            const { stdout } = await execAsync(
                `git log -${count} --format="%H|%s|%aI"`,
                { cwd: this.workspaceRoot }
            );

            return stdout
                .split('\n')
                .filter(line => line.trim())
                .map(line => {
                    const [sha, message, dateStr] = line.split('|');
                    return {
                        sha,
                        message,
                        date: new Date(dateStr),
                    };
                });
        } catch {
            return [];
        }
    }

    /**
     * Get list of branches for baseline selection
     */
    async getBranches(): Promise<string[]> {
        try {
            const { stdout } = await execAsync(
                'git branch --format="%(refname:short)"',
                { cwd: this.workspaceRoot }
            );

            return stdout.split('\n').filter(line => line.trim());
        } catch {
            return [];
        }
    }

    private getGitRef(baseline: DiffBaseline): string {
        switch (baseline.type) {
            case 'commit':
                return baseline.reference;
            case 'branch':
                return baseline.reference;
            default:
                throw new Error(`Unsupported baseline type for Git: ${baseline.type}`);
        }
    }

    private toRelativePath(filePath: string): string {
        if (path.isAbsolute(filePath)) {
            return path.relative(this.workspaceRoot, filePath);
        }
        return filePath;
    }
}

/**
 * Snapshot-based baseline resolver
 * Retrieves content from locally stored snapshots
 */
export class SnapshotBaselineResolver implements IBaselineResolver {
    private snapshotDir: string;

    constructor(private workspaceRoot: string) {
        this.snapshotDir = path.join(workspaceRoot, '.codemap', 'snapshots');
    }

    async getFileContent(filePath: string, baseline: DiffBaseline): Promise<string | null> {
        if (baseline.type !== 'snapshot') {
            return null;
        }

        try {
            const relativePath = this.toRelativePath(filePath);
            const snapshotPath = path.join(this.snapshotDir, baseline.reference, relativePath);

            const content = await fs.promises.readFile(snapshotPath, 'utf-8');
            return content;
        } catch {
            return null;
        }
    }

    async fileExistsInBaseline(filePath: string, baseline: DiffBaseline): Promise<boolean> {
        if (baseline.type !== 'snapshot') {
            return false;
        }

        try {
            const relativePath = this.toRelativePath(filePath);
            const snapshotPath = path.join(this.snapshotDir, baseline.reference, relativePath);

            await fs.promises.access(snapshotPath, fs.constants.F_OK);
            return true;
        } catch {
            return false;
        }
    }

    async listFilesInBaseline(baseline: DiffBaseline): Promise<string[]> {
        if (baseline.type !== 'snapshot') {
            return [];
        }

        try {
            const snapshotPath = path.join(this.snapshotDir, baseline.reference);
            return await this.listFilesRecursive(snapshotPath, '');
        } catch {
            return [];
        }
    }

    /**
     * Create a new snapshot of the current state
     */
    async createSnapshot(label?: string): Promise<DiffBaseline> {
        const snapshotId = this.generateSnapshotId();
        const snapshotPath = path.join(this.snapshotDir, snapshotId);

        // Ensure snapshot directory exists
        await fs.promises.mkdir(snapshotPath, { recursive: true });

        // Copy all tracked files to snapshot
        // This is a simplified implementation - in production,
        // you'd want to use glob patterns from config
        const files = await this.listWorkspaceFiles();

        for (const file of files) {
            const sourcePath = path.join(this.workspaceRoot, file);
            const destPath = path.join(snapshotPath, file);

            // Ensure parent directory exists
            await fs.promises.mkdir(path.dirname(destPath), { recursive: true });

            // Copy file
            await fs.promises.copyFile(sourcePath, destPath);
        }

        // Save metadata
        const metadata = {
            id: snapshotId,
            timestamp: new Date().toISOString(),
            label: label || `Snapshot ${snapshotId}`,
            fileCount: files.length,
        };

        await fs.promises.writeFile(
            path.join(snapshotPath, '.snapshot-meta.json'),
            JSON.stringify(metadata, null, 2)
        );

        return {
            type: 'snapshot',
            reference: snapshotId,
            timestamp: new Date(),
            label: metadata.label,
        };
    }

    /**
     * List all available snapshots
     */
    async listSnapshots(): Promise<DiffBaseline[]> {
        try {
            const entries = await fs.promises.readdir(this.snapshotDir, { withFileTypes: true });
            const snapshots: DiffBaseline[] = [];

            for (const entry of entries) {
                if (entry.isDirectory()) {
                    const metaPath = path.join(this.snapshotDir, entry.name, '.snapshot-meta.json');
                    try {
                        const metaContent = await fs.promises.readFile(metaPath, 'utf-8');
                        const meta = JSON.parse(metaContent);
                        snapshots.push({
                            type: 'snapshot',
                            reference: entry.name,
                            timestamp: new Date(meta.timestamp),
                            label: meta.label,
                        });
                    } catch {
                        // Invalid snapshot, skip
                    }
                }
            }

            // Sort by timestamp descending
            snapshots.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());

            return snapshots;
        } catch {
            return [];
        }
    }

    /**
     * Delete a snapshot
     */
    async deleteSnapshot(snapshotId: string): Promise<void> {
        const snapshotPath = path.join(this.snapshotDir, snapshotId);
        await fs.promises.rm(snapshotPath, { recursive: true, force: true });
    }

    private toRelativePath(filePath: string): string {
        if (path.isAbsolute(filePath)) {
            return path.relative(this.workspaceRoot, filePath);
        }
        return filePath;
    }

    private generateSnapshotId(): string {
        const now = new Date();
        const dateStr = now.toISOString().replace(/[:.]/g, '-').slice(0, 19);
        const random = Math.random().toString(36).substring(2, 8);
        return `${dateStr}-${random}`;
    }

    private async listFilesRecursive(basePath: string, relativePath: string): Promise<string[]> {
        const files: string[] = [];
        const fullPath = path.join(basePath, relativePath);

        try {
            const entries = await fs.promises.readdir(fullPath, { withFileTypes: true });

            for (const entry of entries) {
                if (entry.name.startsWith('.')) continue; // Skip hidden files

                const entryRelative = path.join(relativePath, entry.name);

                if (entry.isDirectory()) {
                    const subFiles = await this.listFilesRecursive(basePath, entryRelative);
                    files.push(...subFiles);
                } else {
                    files.push(entryRelative);
                }
            }
        } catch {
            // Directory doesn't exist or not readable
        }

        return files;
    }

    private async listWorkspaceFiles(): Promise<string[]> {
        // Use VS Code API to respect .gitignore etc.
        const files = await vscode.workspace.findFiles(
            '**/*.{ts,tsx,js,jsx,py,cs,rs,go}',
            '**/node_modules/**'
        );

        return files.map(uri => path.relative(this.workspaceRoot, uri.fsPath));
    }
}

/**
 * Combined resolver that delegates to appropriate resolver based on baseline type
 */
export class BaselineResolver implements IBaselineResolver {
    private gitResolver: GitBaselineResolver;
    private snapshotResolver: SnapshotBaselineResolver;
    private worktreeResolver?: IBaselineResolver;

    constructor(workspaceRoot: string) {
        this.gitResolver = new GitBaselineResolver(workspaceRoot);
        this.snapshotResolver = new SnapshotBaselineResolver(workspaceRoot);
    }

    /**
     * Set the worktree resolver (optional, injected when WorktreeManager is available)
     */
    setWorktreeResolver(resolver: IBaselineResolver): void {
        this.worktreeResolver = resolver;
    }

    async getFileContent(filePath: string, baseline: DiffBaseline): Promise<string | null> {
        switch (baseline.type) {
            case 'commit':
            case 'branch':
                return this.gitResolver.getFileContent(filePath, baseline);
            case 'snapshot':
                return this.snapshotResolver.getFileContent(filePath, baseline);
            case 'worktree':
                if (!this.worktreeResolver) {
                    console.warn('Worktree resolver not configured');
                    return null;
                }
                return this.worktreeResolver.getFileContent(filePath, baseline);
            case 'working-tree':
                // Working tree = no baseline, return null
                return null;
        }
    }

    async fileExistsInBaseline(filePath: string, baseline: DiffBaseline): Promise<boolean> {
        switch (baseline.type) {
            case 'commit':
            case 'branch':
                return this.gitResolver.fileExistsInBaseline(filePath, baseline);
            case 'snapshot':
                return this.snapshotResolver.fileExistsInBaseline(filePath, baseline);
            case 'worktree':
                if (!this.worktreeResolver) {
                    return false;
                }
                return this.worktreeResolver.fileExistsInBaseline(filePath, baseline);
            case 'working-tree':
                return false;
        }
    }

    async listFilesInBaseline(baseline: DiffBaseline): Promise<string[]> {
        switch (baseline.type) {
            case 'commit':
            case 'branch':
                return this.gitResolver.listFilesInBaseline(baseline);
            case 'snapshot':
                return this.snapshotResolver.listFilesInBaseline(baseline);
            case 'worktree':
                if (!this.worktreeResolver) {
                    return [];
                }
                return this.worktreeResolver.listFilesInBaseline(baseline);
            case 'working-tree':
                return [];
        }
    }

    // Expose git-specific methods
    get git(): GitBaselineResolver {
        return this.gitResolver;
    }

    // Expose snapshot-specific methods
    get snapshot(): SnapshotBaselineResolver {
        return this.snapshotResolver;
    }
}
