/**
 * DiffSummaryPopup - Shows diff summary on hover over changed nodes
 */

import type { NodeDiffView } from '../../graph/types';

export interface DiffSummaryPopup {
    show(nodeId: string, position: { x: number; y: number }, diff: NodeDiffView): void;
    hide(): void;
    isVisible(): boolean;
}

export class DiffSummaryPopupImpl implements DiffSummaryPopup {
    private element: HTMLDivElement;
    private hideTimeout: number | null = null;
    private visible: boolean = false;

    constructor(container: HTMLElement) {
        this.element = document.createElement('div');
        this.element.className = 'diff-summary-popup';
        this.element.style.display = 'none';
        container.appendChild(this.element);
    }

    show(nodeId: string, position: { x: number; y: number }, diff: NodeDiffView): void {
        if (this.hideTimeout) {
            clearTimeout(this.hideTimeout);
            this.hideTimeout = null;
        }

        this.element.innerHTML = this.renderContent(diff);

        // Position popup near cursor but ensure it stays in viewport
        const popupWidth = 200;
        const popupHeight = 120;
        const padding = 10;

        let left = position.x + padding;
        let top = position.y + padding;

        // Adjust if popup would go off-screen
        if (left + popupWidth > window.innerWidth) {
            left = position.x - popupWidth - padding;
        }
        if (top + popupHeight > window.innerHeight) {
            top = position.y - popupHeight - padding;
        }

        this.element.style.left = `${left}px`;
        this.element.style.top = `${top}px`;
        this.element.style.display = 'block';
        this.visible = true;
    }

    hide(): void {
        // Delay hide slightly to prevent flickering
        this.hideTimeout = window.setTimeout(() => {
            this.element.style.display = 'none';
            this.visible = false;
            this.hideTimeout = null;
        }, 100);
    }

    isVisible(): boolean {
        return this.visible;
    }

    private renderContent(diff: NodeDiffView): string {
        const { summary } = diff;
        const parts: string[] = [];

        if (summary.membersAdded > 0) {
            parts.push(`<span class="added">+${summary.membersAdded} member${summary.membersAdded > 1 ? 's' : ''}</span>`);
        }
        if (summary.membersRemoved > 0) {
            parts.push(`<span class="removed">-${summary.membersRemoved} member${summary.membersRemoved > 1 ? 's' : ''}</span>`);
        }
        if (summary.membersModified > 0) {
            parts.push(`<span class="modified">${summary.membersModified} modified</span>`);
        }

        const membersHtml = parts.length > 0
            ? parts.join(' <span class="separator">·</span> ')
            : '<span class="no-changes">No member changes</span>';

        const linesInfo = `<div class="lines">+${summary.linesAdded} -${summary.linesRemoved} lines</div>`;

        const severityBadge = diff.severity !== 'none'
            ? `<span class="severity severity-${diff.severity}">${diff.severity}</span>`
            : '';

        const statusBadge = `<span class="status status-${diff.status}">${diff.status}</span>`;

        return `
            <div class="diff-summary-header">
                ${statusBadge}
                ${severityBadge}
            </div>
            <div class="diff-summary-members">
                ${membersHtml}
            </div>
            ${linesInfo}
            <div class="diff-summary-hint">Click for details</div>
        `;
    }

    dispose(): void {
        if (this.hideTimeout) {
            clearTimeout(this.hideTimeout);
        }
        this.element.remove();
    }
}
