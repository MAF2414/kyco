/**
 * MultiAgentOverview - Webview component for displaying multiple agent worktrees
 * Shows a grid view of all agent worktrees with their status and stats
 */

import type { WorktreeInfo, AgentSession } from '../../diff/types';

/**
 * View modes for multi-agent display
 */
export type ViewMode = 'overview' | 'comparison';

/**
 * Callbacks for overview actions
 */
export interface MultiAgentOverviewCallbacks {
    onSelectAsBaseline: (worktreePath: string) => void;
    onOpenInWindow: (worktreePath: string) => void;
    onRemoveWorktree: (worktreePath: string) => void;
    onCompareWorktrees: (pathA: string, pathB: string) => void;
    onMergeAllReady: () => void;
    onCleanupCompleted: () => void;
}

/**
 * Interface for MultiAgentOverview
 */
export interface IMultiAgentOverview {
    render(worktrees: WorktreeInfo[], sessions: AgentSession[]): string;
    renderComparison(worktreeA: WorktreeInfo, worktreeB: WorktreeInfo): string;
    attachEventHandlers(container: HTMLElement): void;
}

/**
 * MultiAgentOverview implementation
 */
export class MultiAgentOverview implements IMultiAgentOverview {
    private callbacks: MultiAgentOverviewCallbacks;
    private selectedForComparison: string | null = null;

    constructor(callbacks: MultiAgentOverviewCallbacks) {
        this.callbacks = callbacks;
    }

    /**
     * Render the overview grid
     */
    render(worktrees: WorktreeInfo[], sessions: AgentSession[]): string {
        const agentWorktrees = worktrees.filter(wt => !wt.isMain);

        if (agentWorktrees.length === 0) {
            return `
                <div class="multi-agent-overview empty">
                    <div class="empty-state">
                        <span class="empty-icon">🤖</span>
                        <h3>No Agent Worktrees</h3>
                        <p>Agent worktrees will appear here when agents work in isolated mode.</p>
                    </div>
                </div>
            `;
        }

        return `
            <div class="multi-agent-overview">
                <div class="overview-header">
                    <h3>Agent Worktrees</h3>
                    <span class="overview-count">${agentWorktrees.length} active</span>
                </div>

                <div class="overview-grid">
                    ${agentWorktrees.map(wt => this.renderWorktreeCard(wt, sessions)).join('')}
                </div>

                <div class="overview-actions">
                    <button class="overview-action-btn" data-action="merge-all-ready">
                        Merge All Ready
                    </button>
                    <button class="overview-action-btn secondary" data-action="cleanup-completed">
                        Cleanup Completed
                    </button>
                </div>
            </div>
        `;
    }

    /**
     * Render a single worktree card
     */
    private renderWorktreeCard(worktree: WorktreeInfo, sessions: AgentSession[]): string {
        const session = sessions.find(s => s.worktree?.path === worktree.path);
        const agent = worktree.agent;
        const statusClass = agent?.status || 'unknown';

        return `
            <div class="worktree-card ${statusClass}"
                 data-worktree-path="${this.escapeAttr(worktree.path)}">
                <div class="card-header">
                    <span class="card-icon">${this.getStatusIcon(agent?.status)}</span>
                    <span class="card-name">${worktree.name}</span>
                    ${this.selectedForComparison === worktree.path
                        ? '<span class="compare-badge">Selected for Compare</span>'
                        : ''
                    }
                </div>

                ${agent ? `
                    <div class="card-task">${agent.taskDescription}</div>
                    <div class="card-time">${this.formatRelativeTime(agent.createdAt)}</div>
                ` : ''}

                ${session?.pullRequest ? `
                    <div class="card-pr">
                        <span class="pr-icon">#${session.pullRequest.number}</span>
                        <span class="pr-status ${session.pullRequest.status}">${session.pullRequest.status}</span>
                    </div>
                ` : ''}

                <div class="card-stats">
                    <span class="stat">
                        <span class="stat-icon">📝</span>
                        ${worktree.stats.filesModified} files
                    </span>
                    <span class="stat">
                        <span class="stat-icon">↑</span>
                        ${worktree.stats.aheadOfMain} commits
                    </span>
                    ${worktree.stats.behindMain > 0 ? `
                        <span class="stat behind">
                            <span class="stat-icon">↓</span>
                            ${worktree.stats.behindMain} behind
                        </span>
                    ` : ''}
                </div>

                <div class="card-actions">
                    <button class="card-action-btn"
                            data-action="select-baseline"
                            data-worktree-path="${this.escapeAttr(worktree.path)}">
                        Compare
                    </button>
                    <button class="card-action-btn"
                            data-action="open-window"
                            data-worktree-path="${this.escapeAttr(worktree.path)}">
                        Open
                    </button>
                    <button class="card-action-btn danger"
                            data-action="remove"
                            data-worktree-path="${this.escapeAttr(worktree.path)}">
                        ✕
                    </button>
                </div>
            </div>
        `;
    }

    /**
     * Render comparison view between two worktrees
     */
    renderComparison(worktreeA: WorktreeInfo, worktreeB: WorktreeInfo): string {
        return `
            <div class="worktree-comparison">
                <div class="comparison-header">
                    <h3>Comparing Worktrees</h3>
                    <button class="close-btn" data-action="close-comparison">✕</button>
                </div>

                <div class="comparison-sides">
                    <div class="comparison-side">
                        <div class="side-header">
                            <span class="side-icon">${worktreeA.isMain ? '🏠' : '🤖'}</span>
                            <span class="side-name">${worktreeA.name}</span>
                        </div>
                        <div class="side-branch">${worktreeA.branch}</div>
                        <div class="side-stats">
                            ${worktreeA.stats.filesModified} modified files
                        </div>
                    </div>

                    <div class="comparison-arrow">⟷</div>

                    <div class="comparison-side">
                        <div class="side-header">
                            <span class="side-icon">${worktreeB.isMain ? '🏠' : '🤖'}</span>
                            <span class="side-name">${worktreeB.name}</span>
                        </div>
                        <div class="side-branch">${worktreeB.branch}</div>
                        <div class="side-stats">
                            ${worktreeB.stats.filesModified} modified files
                        </div>
                    </div>
                </div>

                <div class="comparison-actions">
                    <button class="comparison-action-btn primary"
                            data-action="run-comparison"
                            data-path-a="${this.escapeAttr(worktreeA.path)}"
                            data-path-b="${this.escapeAttr(worktreeB.path)}">
                        Run Comparison
                    </button>
                </div>
            </div>
        `;
    }

    /**
     * Attach event handlers
     */
    attachEventHandlers(container: HTMLElement): void {
        // Select as baseline
        container.querySelectorAll('[data-action="select-baseline"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) {
                    if (this.selectedForComparison && this.selectedForComparison !== path) {
                        // Second selection - start comparison
                        this.callbacks.onCompareWorktrees(this.selectedForComparison, path);
                        this.selectedForComparison = null;
                    } else if (this.selectedForComparison === path) {
                        // Deselect
                        this.selectedForComparison = null;
                    } else {
                        // First selection
                        this.selectedForComparison = path;
                        this.callbacks.onSelectAsBaseline(path);
                    }
                    this.updateSelectionUI(container);
                }
            });
        });

        // Open in window
        container.querySelectorAll('[data-action="open-window"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) this.callbacks.onOpenInWindow(path);
            });
        });

        // Remove worktree
        container.querySelectorAll('[data-action="remove"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) this.callbacks.onRemoveWorktree(path);
            });
        });

        // Merge all ready
        container.querySelector('[data-action="merge-all-ready"]')?.addEventListener('click', () => {
            this.callbacks.onMergeAllReady();
        });

        // Cleanup completed
        container.querySelector('[data-action="cleanup-completed"]')?.addEventListener('click', () => {
            this.callbacks.onCleanupCompleted();
        });

        // Run comparison
        container.querySelectorAll('[data-action="run-comparison"]').forEach(el => {
            el.addEventListener('click', () => {
                const pathA = (el as HTMLElement).dataset.pathA;
                const pathB = (el as HTMLElement).dataset.pathB;
                if (pathA && pathB) {
                    this.callbacks.onCompareWorktrees(pathA, pathB);
                }
            });
        });
    }

    /**
     * Update selection UI
     */
    private updateSelectionUI(container: HTMLElement): void {
        container.querySelectorAll('.worktree-card').forEach(card => {
            const path = (card as HTMLElement).dataset.worktreePath;
            const badge = card.querySelector('.compare-badge');

            if (path === this.selectedForComparison) {
                if (!badge) {
                    const header = card.querySelector('.card-header');
                    if (header) {
                        const newBadge = document.createElement('span');
                        newBadge.className = 'compare-badge';
                        newBadge.textContent = 'Selected for Compare';
                        header.appendChild(newBadge);
                    }
                }
            } else {
                badge?.remove();
            }
        });
    }

    // Helper methods
    private getStatusIcon(status?: 'active' | 'completed' | 'abandoned'): string {
        switch (status) {
            case 'active': return '🔄';
            case 'completed': return '✅';
            case 'abandoned': return '⚠️';
            default: return '🤖';
        }
    }

    private formatRelativeTime(date: Date): string {
        const now = new Date();
        const diff = now.getTime() - date.getTime();
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(diff / 3600000);
        const days = Math.floor(diff / 86400000);

        if (minutes < 1) return 'just now';
        if (minutes < 60) return `${minutes}m ago`;
        if (hours < 24) return `${hours}h ago`;
        if (days < 7) return `${days}d ago`;
        if (days < 30) return `${Math.floor(days / 7)}w ago`;
        return date.toLocaleDateString();
    }

    private escapeAttr(str: string): string {
        return str.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
    }
}

/**
 * Factory function
 */
export function createMultiAgentOverview(
    callbacks: MultiAgentOverviewCallbacks
): MultiAgentOverview {
    return new MultiAgentOverview(callbacks);
}
