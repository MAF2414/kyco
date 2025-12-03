/**
 * WorktreeManager - Discovery and management of Git worktrees
 * Handles worktree listing, creation, removal, and file access
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { exec } from 'child_process';
import { promisify } from 'util';
import type {
    WorktreeInfo,
    WorktreeStats,
    WorktreeDiff,
    WorktreeFileDiff,
    AgentWorktreeMetadata,
} from './types';

const execAsync = promisify(exec);

/**
 * Simple event emitter for worktree events
 */
class EventEmitter<T> {
    private listeners: ((event: T) => void)[] = [];

    on(listener: (event: T) => void): { dispose: () => void } {
        this.listeners.push(listener);
        return {
            dispose: () => {
                const index = this.listeners.indexOf(listener);
                if (index >= 0) this.listeners.splice(index, 1);
            },
        };
    }

    fire(event: T): void {
        for (const listener of this.listeners) {
            listener(event);
        }
    }
}

/**
 * Interface for WorktreeManager
 */
export interface IWorktreeManager {
    // Discovery
    listWorktrees(): Promise<WorktreeInfo[]>;
    getMainWorktree(): Promise<WorktreeInfo>;
    getCurrentWorktree(): Promise<WorktreeInfo>;
    getWorktree(pathOrName: string): Promise<WorktreeInfo | null>;

    // CRUD
    createWorktree(name: string, baseBranch?: string): Promise<WorktreeInfo>;
    removeWorktree(pathOrName: string, force?: boolean): Promise<void>;

    // File Access
    getFileContent(worktreePath: string, filePath: string): Promise<string | null>;
    listFiles(worktreePath: string, glob?: string): Promise<string[]>;

    // Comparison
    compareWorktrees(worktreeA: string, worktreeB: string): Promise<WorktreeDiff>;

    // Events
    onWorktreeChanged: EventEmitter<WorktreeInfo>;
    onWorktreeCreated: EventEmitter<WorktreeInfo>;
    onWorktreeRemoved: EventEmitter<string>;

    // Cleanup
    dispose(): void;
}

/**
 * WorktreeManager implementation
 */
export class WorktreeManager implements IWorktreeManager {
    public onWorktreeChanged = new EventEmitter<WorktreeInfo>();
    public onWorktreeCreated = new EventEmitter<WorktreeInfo>();
    public onWorktreeRemoved = new EventEmitter<string>();

    private worktreeCache: Map<string, WorktreeInfo> = new Map();
    private cacheTimestamp: number = 0;
    private readonly CACHE_TTL_MS = 5000; // 5 seconds

    constructor(private workspaceRoot: string) {}

    /**
     * List all Git worktrees
     */
    async listWorktrees(): Promise<WorktreeInfo[]> {
        // Check cache
        if (Date.now() - this.cacheTimestamp < this.CACHE_TTL_MS && this.worktreeCache.size > 0) {
            return Array.from(this.worktreeCache.values());
        }

        try {
            // git worktree list --porcelain
            const { stdout } = await execAsync(
                'git worktree list --porcelain',
                { cwd: this.workspaceRoot }
            );

            const partialWorktrees = this.parseWorktreeList(stdout);

            // Enrich with stats
            const worktrees = await Promise.all(
                partialWorktrees.map(wt => this.enrichWorktreeInfo(wt))
            );

            // Update cache
            this.worktreeCache.clear();
            for (const wt of worktrees) {
                this.worktreeCache.set(wt.path, wt);
            }
            this.cacheTimestamp = Date.now();

            return worktrees;
        } catch (error) {
            console.warn('Failed to list worktrees:', error);
            return [];
        }
    }

    /**
     * Parse git worktree list --porcelain output
     */
    private parseWorktreeList(output: string): Partial<WorktreeInfo>[] {
        const worktrees: Partial<WorktreeInfo>[] = [];
        let current: Partial<WorktreeInfo> = {};

        for (const line of output.split('\n')) {
            if (line.startsWith('worktree ')) {
                if (current.path) worktrees.push(current);
                current = { path: line.substring(9) };
            } else if (line.startsWith('HEAD ')) {
                current.head = line.substring(5);
            } else if (line.startsWith('branch ')) {
                current.branch = line.substring(7).replace('refs/heads/', '');
            } else if (line === 'bare') {
                current.isBare = true;
            } else if (line === 'locked') {
                current.locked = true;
            } else if (line === 'prunable') {
                current.prunable = true;
            }
        }

        if (current.path) worktrees.push(current);
        return worktrees;
    }

    /**
     * Enrich partial worktree info with stats and metadata
     */
    private async enrichWorktreeInfo(partial: Partial<WorktreeInfo>): Promise<WorktreeInfo> {
        const wtPath = partial.path!;
        const isMain = path.normalize(wtPath) === path.normalize(this.workspaceRoot);
        const name = isMain ? 'main' : path.basename(wtPath);

        // Stats berechnen
        const stats = await this.calculateWorktreeStats(wtPath);

        // Agent-Metadaten laden falls vorhanden
        const agent = await this.loadAgentMetadata(wtPath);

        return {
            path: wtPath,
            name,
            branch: partial.branch || 'detached',
            head: partial.head || 'unknown',
            isMain,
            isBare: partial.isBare || false,
            locked: partial.locked || false,
            prunable: partial.prunable || false,
            agent,
            stats,
        };
    }

    /**
     * Calculate worktree statistics
     */
    private async calculateWorktreeStats(worktreePath: string): Promise<WorktreeStats> {
        let filesModified = 0;
        let uncommittedChanges = false;
        let aheadOfMain = 0;
        let behindMain = 0;

        try {
            // Uncommitted changes - git status --porcelain
            const { stdout: statusOutput } = await execAsync(
                'git status --porcelain',
                { cwd: worktreePath }
            );
            const modifiedFiles = statusOutput.split('\n').filter(line => line.trim());
            filesModified = modifiedFiles.length;
            uncommittedChanges = filesModified > 0;

            // Ahead/behind main
            const mainBranch = await this.getMainBranchName();
            try {
                const { stdout: revList } = await execAsync(
                    `git rev-list --left-right --count ${mainBranch}...HEAD`,
                    { cwd: worktreePath }
                );
                const [behind, ahead] = revList.trim().split(/\s+/).map(Number);
                aheadOfMain = ahead || 0;
                behindMain = behind || 0;
            } catch {
                // Branch might not track main
            }
        } catch (error) {
            console.warn(`Failed to calculate stats for ${worktreePath}:`, error);
        }

        return { filesModified, uncommittedChanges, aheadOfMain, behindMain };
    }

    /**
     * Load agent metadata from worktree if it exists
     */
    private async loadAgentMetadata(worktreePath: string): Promise<WorktreeInfo['agent'] | undefined> {
        const metadataPath = path.join(worktreePath, '.codemap', 'agent.json');

        try {
            const content = await fs.promises.readFile(metadataPath, 'utf-8');
            const metadata: AgentWorktreeMetadata = JSON.parse(content);

            return {
                id: metadata.id,
                taskDescription: metadata.taskDescription,
                createdAt: new Date(metadata.createdAt),
                status: metadata.status,
            };
        } catch {
            return undefined;
        }
    }

    /**
     * Get the main branch name (main or master)
     */
    private async getMainBranchName(): Promise<string> {
        try {
            await execAsync('git rev-parse --verify main', { cwd: this.workspaceRoot });
            return 'main';
        } catch {
            return 'master';
        }
    }

    /**
     * Get the main worktree
     */
    async getMainWorktree(): Promise<WorktreeInfo> {
        const worktrees = await this.listWorktrees();
        const main = worktrees.find(wt => wt.isMain);
        if (!main) {
            throw new Error('No main worktree found');
        }
        return main;
    }

    /**
     * Get the current worktree (based on workspaceRoot)
     */
    async getCurrentWorktree(): Promise<WorktreeInfo> {
        const worktrees = await this.listWorktrees();
        const current = worktrees.find(wt =>
            path.normalize(wt.path) === path.normalize(this.workspaceRoot)
        );
        if (!current) {
            throw new Error('Current worktree not found');
        }
        return current;
    }

    /**
     * Get a specific worktree by path or name
     */
    async getWorktree(pathOrName: string): Promise<WorktreeInfo | null> {
        const worktrees = await this.listWorktrees();

        // Try exact path match first
        let found = worktrees.find(wt =>
            path.normalize(wt.path) === path.normalize(pathOrName)
        );

        // Then try name match
        if (!found) {
            found = worktrees.find(wt => wt.name === pathOrName);
        }

        return found || null;
    }

    /**
     * Create a new worktree
     */
    async createWorktree(name: string, baseBranch?: string): Promise<WorktreeInfo> {
        // Determine worktree location
        const worktreesDir = path.join(path.dirname(this.workspaceRoot), '.worktrees');
        const worktreePath = path.join(worktreesDir, name);
        const branch = `agent/${name}`;
        const base = baseBranch || await this.getMainBranchName();

        // Ensure parent directory exists
        await fs.promises.mkdir(worktreesDir, { recursive: true });

        // Create worktree with new branch
        await execAsync(
            `git worktree add -b "${branch}" "${worktreePath}" "${base}"`,
            { cwd: this.workspaceRoot }
        );

        // Invalidate cache
        this.cacheTimestamp = 0;

        // Get and return the new worktree info
        const worktree = await this.getWorktree(worktreePath);
        if (!worktree) {
            throw new Error(`Failed to create worktree at ${worktreePath}`);
        }

        this.onWorktreeCreated.fire(worktree);
        return worktree;
    }

    /**
     * Remove a worktree
     */
    async removeWorktree(pathOrName: string, force: boolean = false): Promise<void> {
        const worktree = await this.getWorktree(pathOrName);
        if (!worktree) {
            throw new Error(`Worktree not found: ${pathOrName}`);
        }

        if (worktree.isMain) {
            throw new Error('Cannot remove main worktree');
        }

        const forceFlag = force ? '--force' : '';

        // Remove worktree
        await execAsync(
            `git worktree remove ${forceFlag} "${worktree.path}"`,
            { cwd: this.workspaceRoot }
        );

        // Delete the branch if it's an agent branch
        if (worktree.branch.startsWith('agent/')) {
            try {
                await execAsync(
                    `git branch -D "${worktree.branch}"`,
                    { cwd: this.workspaceRoot }
                );
            } catch {
                // Branch might not exist or already deleted
            }
        }

        // Invalidate cache
        this.cacheTimestamp = 0;
        this.worktreeCache.delete(worktree.path);

        this.onWorktreeRemoved.fire(worktree.path);
    }

    /**
     * Get file content from a worktree
     */
    async getFileContent(worktreePath: string, filePath: string): Promise<string | null> {
        const fullPath = path.isAbsolute(filePath)
            ? filePath
            : path.join(worktreePath, filePath);

        try {
            return await fs.promises.readFile(fullPath, 'utf-8');
        } catch {
            return null;
        }
    }

    /**
     * List files in a worktree
     */
    async listFiles(worktreePath: string, globPattern?: string): Promise<string[]> {
        try {
            // Use git ls-files for tracked files
            const { stdout } = await execAsync(
                'git ls-files',
                { cwd: worktreePath, maxBuffer: 10 * 1024 * 1024 }
            );

            let files = stdout.split('\n').filter(line => line.trim());

            // Apply simple glob filter if provided
            if (globPattern) {
                const regex = this.globToRegex(globPattern);
                files = files.filter(file => regex.test(file));
            }

            return files;
        } catch {
            return [];
        }
    }

    /**
     * Convert simple glob pattern to regex
     */
    private globToRegex(glob: string): RegExp {
        const escaped = glob
            .replace(/[.+^${}()|[\]\\]/g, '\\$&')  // Escape special regex chars except * and ?
            .replace(/\*\*/g, '{{GLOBSTAR}}')      // Temporarily replace **
            .replace(/\*/g, '[^/]*')               // * matches anything except /
            .replace(/\?/g, '.')                   // ? matches single char
            .replace(/{{GLOBSTAR}}/g, '.*');       // ** matches anything including /
        return new RegExp(`^${escaped}$`);
    }

    /**
     * Compare two worktrees
     */
    async compareWorktrees(pathA: string, pathB: string): Promise<WorktreeDiff> {
        const filesA = new Set(await this.listFiles(pathA));
        const filesB = new Set(await this.listFiles(pathB));

        const allFiles = new Set([...filesA, ...filesB]);
        const diffs: WorktreeFileDiff[] = [];

        for (const file of allFiles) {
            const inA = filesA.has(file);
            const inB = filesB.has(file);

            if (inA && !inB) {
                diffs.push({ file, status: 'removed', inWorktree: pathA });
            } else if (!inA && inB) {
                diffs.push({ file, status: 'added', inWorktree: pathB });
            } else {
                // Both have the file - compare contents
                const contentA = await this.getFileContent(pathA, file);
                const contentB = await this.getFileContent(pathB, file);

                if (contentA !== contentB) {
                    diffs.push({ file, status: 'modified' });
                }
            }
        }

        return { worktreeA: pathA, worktreeB: pathB, diffs };
    }

    /**
     * Save agent metadata to worktree
     */
    async saveAgentMetadata(worktreePath: string, metadata: AgentWorktreeMetadata): Promise<void> {
        const metadataDir = path.join(worktreePath, '.codemap');
        const metadataPath = path.join(metadataDir, 'agent.json');

        await fs.promises.mkdir(metadataDir, { recursive: true });
        await fs.promises.writeFile(metadataPath, JSON.stringify(metadata, null, 2));

        // Invalidate cache for this worktree
        this.worktreeCache.delete(worktreePath);
    }

    /**
     * Open worktree in new VS Code window
     */
    async openInNewWindow(pathOrName: string): Promise<void> {
        const worktree = await this.getWorktree(pathOrName);
        if (!worktree) {
            throw new Error(`Worktree not found: ${pathOrName}`);
        }

        const uri = vscode.Uri.file(worktree.path);
        await vscode.commands.executeCommand('vscode.openFolder', uri, { forceNewWindow: true });
    }

    /**
     * Cleanup resources
     */
    dispose(): void {
        this.worktreeCache.clear();
    }
}

/**
 * Factory function for creating WorktreeManager
 */
export function createWorktreeManager(workspaceRoot: string): WorktreeManager {
    return new WorktreeManager(workspaceRoot);
}
