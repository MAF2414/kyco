/**
 * WorktreeBaselineSelectorUI - Webview component for worktree-aware baseline selection
 * Displays worktrees, commits, branches, and snapshots in a dropdown selector
 */

import type {
    WorktreeInfo,
    DiffBaseline,
    AgentSession,
    WorktreeStats,
} from '../../diff/types';

/**
 * Baseline state for rendering
 */
export interface BaselineState {
    current: DiffBaseline | null;
    available: {
        commits: { sha: string; message: string; date: Date }[];
        branches: string[];
        snapshots: DiffBaseline[];
    };
}

/**
 * Interface for WorktreeBaselineSelectorUI
 */
export interface IWorktreeBaselineSelectorUI {
    render(
        state: BaselineState,
        worktrees: WorktreeInfo[],
        sessions: AgentSession[]
    ): string;

    attachEventHandlers(container: HTMLElement): void;
}

/**
 * Callbacks for selector actions
 */
export interface WorktreeSelectorCallbacks {
    onSelectWorktreeBaseline: (worktreePath: string) => void;
    onSelectCommitBaseline: (sha: string) => void;
    onSelectBranchBaseline: (branch: string) => void;
    onSelectSnapshotBaseline: (snapshotId: string) => void;
    onCreateSnapshot: () => void;
    onClearBaseline: () => void;
    onOpenWorktree: (worktreePath: string) => void;
    onRemoveWorktree: (worktreePath: string) => void;
}

/**
 * WorktreeBaselineSelectorUI implementation
 */
export class WorktreeBaselineSelectorUI implements IWorktreeBaselineSelectorUI {
    private callbacks: WorktreeSelectorCallbacks;
    private isDropdownOpen = false;

    constructor(callbacks: WorktreeSelectorCallbacks) {
        this.callbacks = callbacks;
    }

    /**
     * Render the baseline selector
     */
    render(
        state: BaselineState,
        worktrees: WorktreeInfo[],
        sessions: AgentSession[]
    ): string {
        return `
            <div class="baseline-selector">
                ${this.renderCurrentBaseline(state.current)}

                <div class="baseline-dropdown ${this.isDropdownOpen ? '' : 'hidden'}">
                    ${this.renderWorktreeSection(worktrees, sessions)}
                    ${this.renderSection('Commits', state.available.commits.map(c => ({
                        id: c.sha,
                        label: this.truncate(c.message, 40),
                        sublabel: this.formatShortSha(c.sha),
                        date: c.date,
                    })), 'commit')}
                    ${this.renderSection('Branches', state.available.branches.map(b => ({
                        id: b,
                        label: b,
                    })), 'branch')}
                    ${this.renderSection('Snapshots', state.available.snapshots.map(s => ({
                        id: s.reference,
                        label: s.label || s.reference,
                        sublabel: this.formatDate(s.timestamp),
                    })), 'snapshot')}

                    <div class="baseline-actions">
                        <button class="baseline-action-btn" data-action="snapshot">
                            <span class="action-icon">📸</span> Snapshot
                        </button>
                        <button class="baseline-action-btn" data-action="clear">
                            <span class="action-icon">✕</span> Clear
                        </button>
                    </div>
                </div>
            </div>
        `;
    }

    /**
     * Render current baseline badge
     */
    private renderCurrentBaseline(baseline: DiffBaseline | null): string {
        if (!baseline) {
            return `
                <button class="baseline-badge empty" data-action="toggle-dropdown">
                    <span class="baseline-icon">⊘</span>
                    <span class="baseline-text">No baseline</span>
                    <span class="dropdown-arrow">▼</span>
                </button>
            `;
        }

        const icon = this.getBaselineIcon(baseline);
        const label = this.getBaselineLabel(baseline);

        return `
            <button class="baseline-badge" data-action="toggle-dropdown">
                <span class="baseline-icon">${icon}</span>
                <span class="baseline-type">${baseline.type}</span>
                <span class="baseline-text">${label}</span>
                <span class="dropdown-arrow">▼</span>
            </button>
        `;
    }

    /**
     * Render worktree section
     */
    private renderWorktreeSection(
        worktrees: WorktreeInfo[],
        sessions: AgentSession[]
    ): string {
        if (worktrees.length <= 1) return ''; // Only main = no display

        const nonMainWorktrees = worktrees.filter(wt => !wt.isMain);
        const mainWorktree = worktrees.find(wt => wt.isMain);

        return `
            <div class="baseline-section">
                <div class="baseline-section-header">
                    Worktrees
                    <span class="section-badge">${nonMainWorktrees.length}</span>
                </div>
                <div class="baseline-section-items">
                    ${mainWorktree ? this.renderMainWorktree(mainWorktree) : ''}
                    ${nonMainWorktrees.map(wt => this.renderWorktreeItem(wt, sessions)).join('')}
                </div>
            </div>
        `;
    }

    /**
     * Render main worktree item
     */
    private renderMainWorktree(worktree: WorktreeInfo): string {
        return `
            <div class="baseline-item worktree-item main-worktree"
                 data-action="select-worktree"
                 data-worktree-path="${this.escapeAttr(worktree.path)}">
                <span class="baseline-item-icon">🏠</span>
                <div class="baseline-item-content">
                    <span class="baseline-item-label">Main Workspace</span>
                    <span class="baseline-item-branch">${worktree.branch}</span>
                </div>
                ${this.renderWorktreeStats(worktree.stats)}
            </div>
        `;
    }

    /**
     * Render non-main worktree item
     */
    private renderWorktreeItem(worktree: WorktreeInfo, sessions: AgentSession[]): string {
        const session = sessions.find(s => s.worktree?.path === worktree.path);
        const statusIcon = this.getWorktreeStatusIcon(worktree, session);

        return `
            <div class="baseline-item worktree-item"
                 data-action="select-worktree"
                 data-worktree-path="${this.escapeAttr(worktree.path)}">
                <span class="baseline-item-icon">${statusIcon}</span>
                <div class="baseline-item-content">
                    <span class="baseline-item-label">${worktree.name}</span>
                    ${session
                        ? `<span class="baseline-item-task">${this.truncate(session.taskDescription, 30)}</span>`
                        : `<span class="baseline-item-branch">${worktree.branch}</span>`
                    }
                    ${session?.pullRequest
                        ? `<span class="baseline-item-pr">#${session.pullRequest.number} ${session.pullRequest.status}</span>`
                        : ''
                    }
                </div>
                ${this.renderWorktreeStats(worktree.stats)}
                <div class="worktree-actions">
                    <button class="worktree-action-btn"
                            data-action="open-worktree"
                            data-worktree-path="${this.escapeAttr(worktree.path)}"
                            title="Open in new window">
                        ↗
                    </button>
                    <button class="worktree-action-btn"
                            data-action="remove-worktree"
                            data-worktree-path="${this.escapeAttr(worktree.path)}"
                            title="Remove worktree">
                        ×
                    </button>
                </div>
            </div>
        `;
    }

    /**
     * Render worktree stats
     */
    private renderWorktreeStats(stats: WorktreeStats): string {
        const parts: string[] = [];

        if (stats.uncommittedChanges) {
            parts.push(`<span class="stat modified">${stats.filesModified} modified</span>`);
        }
        if (stats.aheadOfMain > 0) {
            parts.push(`<span class="stat ahead">+${stats.aheadOfMain}</span>`);
        }
        if (stats.behindMain > 0) {
            parts.push(`<span class="stat behind">-${stats.behindMain}</span>`);
        }

        if (parts.length === 0) {
            parts.push(`<span class="stat clean">clean</span>`);
        }

        return `<div class="worktree-stats">${parts.join('')}</div>`;
    }

    /**
     * Render a generic section (commits, branches, snapshots)
     */
    private renderSection(
        title: string,
        items: { id: string; label: string; sublabel?: string; date?: Date }[],
        type: 'commit' | 'branch' | 'snapshot'
    ): string {
        if (items.length === 0) return '';

        return `
            <div class="baseline-section">
                <div class="baseline-section-header">${title}</div>
                <div class="baseline-section-items">
                    ${items.slice(0, 10).map(item => `
                        <div class="baseline-item"
                             data-action="select-${type}"
                             data-id="${this.escapeAttr(item.id)}">
                            <span class="baseline-item-icon">${this.getTypeIcon(type)}</span>
                            <div class="baseline-item-content">
                                <span class="baseline-item-label">${item.label}</span>
                                ${item.sublabel ? `<span class="baseline-item-sublabel">${item.sublabel}</span>` : ''}
                            </div>
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    }

    /**
     * Attach event handlers to the rendered DOM
     */
    attachEventHandlers(container: HTMLElement): void {
        // Toggle dropdown
        container.querySelector('[data-action="toggle-dropdown"]')?.addEventListener('click', () => {
            this.isDropdownOpen = !this.isDropdownOpen;
            const dropdown = container.querySelector('.baseline-dropdown');
            dropdown?.classList.toggle('hidden', !this.isDropdownOpen);
        });

        // Close dropdown on outside click
        document.addEventListener('click', (e) => {
            if (!container.contains(e.target as Node)) {
                this.isDropdownOpen = false;
                container.querySelector('.baseline-dropdown')?.classList.add('hidden');
            }
        });

        // Worktree selection
        container.querySelectorAll('[data-action="select-worktree"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) this.callbacks.onSelectWorktreeBaseline(path);
                this.closeDropdown(container);
            });
        });

        // Commit selection
        container.querySelectorAll('[data-action="select-commit"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const id = (el as HTMLElement).dataset.id;
                if (id) this.callbacks.onSelectCommitBaseline(id);
                this.closeDropdown(container);
            });
        });

        // Branch selection
        container.querySelectorAll('[data-action="select-branch"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const id = (el as HTMLElement).dataset.id;
                if (id) this.callbacks.onSelectBranchBaseline(id);
                this.closeDropdown(container);
            });
        });

        // Snapshot selection
        container.querySelectorAll('[data-action="select-snapshot"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const id = (el as HTMLElement).dataset.id;
                if (id) this.callbacks.onSelectSnapshotBaseline(id);
                this.closeDropdown(container);
            });
        });

        // Open worktree
        container.querySelectorAll('[data-action="open-worktree"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) this.callbacks.onOpenWorktree(path);
            });
        });

        // Remove worktree
        container.querySelectorAll('[data-action="remove-worktree"]').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = (el as HTMLElement).dataset.worktreePath;
                if (path) this.callbacks.onRemoveWorktree(path);
            });
        });

        // Create snapshot
        container.querySelector('[data-action="snapshot"]')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.callbacks.onCreateSnapshot();
            this.closeDropdown(container);
        });

        // Clear baseline
        container.querySelector('[data-action="clear"]')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.callbacks.onClearBaseline();
            this.closeDropdown(container);
        });
    }

    private closeDropdown(container: HTMLElement): void {
        this.isDropdownOpen = false;
        container.querySelector('.baseline-dropdown')?.classList.add('hidden');
    }

    // Helper methods
    private getBaselineIcon(baseline: DiffBaseline): string {
        switch (baseline.type) {
            case 'commit': return '📌';
            case 'branch': return '🌿';
            case 'snapshot': return '📸';
            case 'worktree': return baseline.worktree?.isMain ? '🏠' : '🤖';
            default: return '⊘';
        }
    }

    private getBaselineLabel(baseline: DiffBaseline): string {
        switch (baseline.type) {
            case 'commit':
                return this.formatShortSha(baseline.reference);
            case 'worktree':
                return baseline.worktree?.name || baseline.reference;
            default:
                return baseline.label || baseline.reference;
        }
    }

    private getTypeIcon(type: 'commit' | 'branch' | 'snapshot'): string {
        switch (type) {
            case 'commit': return '📌';
            case 'branch': return '🌿';
            case 'snapshot': return '📸';
        }
    }

    private getWorktreeStatusIcon(worktree: WorktreeInfo, session?: AgentSession): string {
        if (session?.pullRequest) {
            switch (session.pullRequest.status) {
                case 'draft': return '📝';
                case 'open': return '🔵';
                case 'merged': return '✅';
                case 'closed': return '❌';
            }
        }

        if (session) {
            return '🤖'; // Agent worktree
        }

        if (worktree.stats.uncommittedChanges) {
            return '🟡'; // Uncommitted changes
        }

        return '🌿'; // Clean worktree
    }

    private truncate(str: string, maxLength: number): string {
        if (str.length <= maxLength) return str;
        return str.slice(0, maxLength - 1) + '…';
    }

    private formatShortSha(sha: string): string {
        return sha.slice(0, 7);
    }

    private formatDate(date: Date): string {
        const now = new Date();
        const diff = now.getTime() - date.getTime();
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(diff / 3600000);
        const days = Math.floor(diff / 86400000);

        if (minutes < 1) return 'just now';
        if (minutes < 60) return `${minutes}m ago`;
        if (hours < 24) return `${hours}h ago`;
        if (days < 7) return `${days}d ago`;
        return date.toLocaleDateString();
    }

    private escapeAttr(str: string): string {
        return str.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
    }
}

/**
 * Factory function
 */
export function createWorktreeBaselineSelectorUI(
    callbacks: WorktreeSelectorCallbacks
): WorktreeBaselineSelectorUI {
    return new WorktreeBaselineSelectorUI(callbacks);
}
