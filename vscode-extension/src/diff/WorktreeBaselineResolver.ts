/**
 * WorktreeBaselineResolver - retrieves file content from Git worktrees
 * Used for comparing code between worktrees (e.g., agent worktree vs main)
 */

import type { DiffBaseline } from './types';
import type { IBaselineResolver } from './BaselineResolver';
import type { IWorktreeManager } from './WorktreeManager';

/**
 * Worktree-based baseline resolver
 * Retrieves content from Git worktrees for baseline comparison
 */
export class WorktreeBaselineResolver implements IBaselineResolver {
    constructor(private worktreeManager: IWorktreeManager) {}

    /**
     * Get file content from a worktree baseline
     */
    async getFileContent(filePath: string, baseline: DiffBaseline): Promise<string | null> {
        if (baseline.type !== 'worktree') {
            throw new Error('WorktreeBaselineResolver only handles worktree baselines');
        }

        if (!baseline.worktree) {
            throw new Error('Worktree baseline missing worktree information');
        }

        return this.worktreeManager.getFileContent(baseline.worktree.path, filePath);
    }

    /**
     * Check if file exists in worktree baseline
     */
    async fileExistsInBaseline(filePath: string, baseline: DiffBaseline): Promise<boolean> {
        if (baseline.type !== 'worktree') {
            return false;
        }

        const content = await this.getFileContent(filePath, baseline);
        return content !== null;
    }

    /**
     * List all files in the worktree baseline
     */
    async listFilesInBaseline(baseline: DiffBaseline): Promise<string[]> {
        if (baseline.type !== 'worktree') {
            throw new Error('WorktreeBaselineResolver only handles worktree baselines');
        }

        if (!baseline.worktree) {
            throw new Error('Worktree baseline missing worktree information');
        }

        return this.worktreeManager.listFiles(baseline.worktree.path);
    }
}

/**
 * Factory function for creating WorktreeBaselineResolver
 */
export function createWorktreeBaselineResolver(
    worktreeManager: IWorktreeManager
): WorktreeBaselineResolver {
    return new WorktreeBaselineResolver(worktreeManager);
}
