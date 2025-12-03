/**
 * DiffDetailPanel - Shows detailed structural diff for a selected node
 */

import type { NodeDiffView, MemberDiffView } from '../../graph/types';

export interface DiffDetailPanel {
    open(nodeId: string, diff: NodeDiffView): void;
    close(): void;
    isOpen(): boolean;
    update(diff: NodeDiffView): void;
    getCurrentNodeId(): string | null;
    onOpenInEditor?: (nodeId: string) => void;
    onRevertNode?: (nodeId: string) => void;
}

function escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

export class DiffDetailPanelImpl implements DiffDetailPanel {
    private panel: HTMLDivElement;
    private currentNodeId: string | null = null;
    private graphContainer: HTMLElement | null = null;

    public onOpenInEditor?: (nodeId: string) => void;
    public onRevertNode?: (nodeId: string) => void;

    constructor(container: HTMLElement, graphContainer?: HTMLElement) {
        this.graphContainer = graphContainer || null;
        this.panel = document.createElement('div');
        this.panel.className = 'diff-detail-panel';
        this.panel.style.display = 'none';
        container.appendChild(this.panel);

        // Set up event delegation for panel actions
        this.panel.addEventListener('click', this.handleClick.bind(this));
    }

    private handleClick(event: Event): void {
        const target = event.target as HTMLElement;

        if (target.classList.contains('diff-panel-close')) {
            this.close();
        } else if (target.dataset.action === 'open-editor' && this.currentNodeId) {
            this.onOpenInEditor?.(this.currentNodeId);
        } else if (target.dataset.action === 'revert' && this.currentNodeId) {
            this.onRevertNode?.(this.currentNodeId);
        }
    }

    open(nodeId: string, diff: NodeDiffView): void {
        this.currentNodeId = nodeId;
        this.panel.innerHTML = this.renderPanel(nodeId, diff);
        this.panel.style.display = 'flex';

        // Shrink graph container to make room for panel
        this.graphContainer?.classList.add('with-diff-panel');
    }

    close(): void {
        this.panel.style.display = 'none';
        this.currentNodeId = null;
        this.graphContainer?.classList.remove('with-diff-panel');
    }

    isOpen(): boolean {
        return this.currentNodeId !== null;
    }

    getCurrentNodeId(): string | null {
        return this.currentNodeId;
    }

    update(diff: NodeDiffView): void {
        if (this.currentNodeId) {
            this.panel.innerHTML = this.renderPanel(this.currentNodeId, diff);
        }
    }

    private renderPanel(nodeId: string, diff: NodeDiffView): string {
        const severityBadge = diff.severity !== 'none'
            ? `<span class="severity severity-${diff.severity}">${diff.severity}</span>`
            : '';

        return `
            <div class="diff-panel-header">
                <span class="diff-panel-title" title="${escapeHtml(nodeId)}">${escapeHtml(diff.nodeId)}</span>
                ${severityBadge}
                <button class="diff-panel-close" title="Close panel">&times;</button>
            </div>

            <div class="diff-panel-summary">
                <div class="stat">
                    <span class="stat-value added">+${diff.summary.membersAdded}</span>
                    <span class="stat-label">added</span>
                </div>
                <div class="stat">
                    <span class="stat-value removed">-${diff.summary.membersRemoved}</span>
                    <span class="stat-label">removed</span>
                </div>
                <div class="stat">
                    <span class="stat-value modified">${diff.summary.membersModified}</span>
                    <span class="stat-label">modified</span>
                </div>
                <div class="stat">
                    <span class="stat-value lines">+${diff.summary.linesAdded} -${diff.summary.linesRemoved}</span>
                    <span class="stat-label">lines</span>
                </div>
            </div>

            <div class="diff-panel-members">
                ${this.renderMemberDiffs(diff.memberDiffs)}
            </div>

            <div class="diff-panel-actions">
                <button class="panel-action-btn" data-action="open-editor">Open in Editor</button>
            </div>
        `;
    }

    private renderMemberDiffs(members: MemberDiffView[]): string {
        if (members.length === 0) {
            return '<div class="no-changes">No member changes</div>';
        }

        // Group by changeType
        const added = members.filter(m => m.changeType === 'added');
        const removed = members.filter(m => m.changeType === 'removed');
        const modified = members.filter(m => m.changeType === 'modified');

        let html = '';

        if (added.length > 0) {
            html += `
                <div class="member-group">
                    <div class="member-group-header added">Added (${added.length})</div>
                    ${added.map(m => this.renderMember(m)).join('')}
                </div>
            `;
        }

        if (removed.length > 0) {
            html += `
                <div class="member-group">
                    <div class="member-group-header removed">Removed (${removed.length})</div>
                    ${removed.map(m => this.renderMember(m)).join('')}
                </div>
            `;
        }

        if (modified.length > 0) {
            html += `
                <div class="member-group">
                    <div class="member-group-header modified">Modified (${modified.length})</div>
                    ${modified.map(m => this.renderMember(m)).join('')}
                </div>
            `;
        }

        return html;
    }

    private renderMember(member: MemberDiffView): string {
        const icon = this.getMemberIcon(member.type);

        let signatureHtml = '';
        if (member.signatureChange) {
            signatureHtml = `
                <div class="signature-change">
                    <div class="before"><span class="label">-</span> ${escapeHtml(member.signatureChange.before)}</div>
                    <div class="after"><span class="label">+</span> ${escapeHtml(member.signatureChange.after)}</div>
                </div>
            `;
        }

        const linesHtml = (member.linesAdded > 0 || member.linesRemoved > 0)
            ? `<span class="member-lines">+${member.linesAdded} -${member.linesRemoved}</span>`
            : '';

        return `
            <div class="member-diff member-${member.changeType}">
                <div class="member-header">
                    <span class="member-icon">${icon}</span>
                    <span class="member-name">${escapeHtml(member.name)}</span>
                    <span class="member-severity severity-${member.severity}">${member.severity}</span>
                    ${linesHtml}
                </div>
                ${signatureHtml}
            </div>
        `;
    }

    private getMemberIcon(type: MemberDiffView['type']): string {
        switch (type) {
            case 'method':
                return 'f';
            case 'property':
                return '&#9675;'; // ○
            case 'constructor':
                return '&#9670;'; // ◆
            case 'getter':
                return 'g';
            case 'setter':
                return 's';
            default:
                return '&#8226;'; // •
        }
    }

    dispose(): void {
        this.panel.remove();
    }
}
