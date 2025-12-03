/**
 * HealthbarRenderer - Renders healthbars below nodes to show diff status
 * Uses vis-network's afterDrawing callback for custom canvas rendering
 */

import type { Network, DataSet, Node } from 'vis-network/standalone';
import type { NodeDiffView } from '../../graph/types';

export interface HealthbarConfig {
    width: number;      // Relative to node width (0.0 - 1.0)
    height: number;     // Pixels
    offsetY: number;    // Pixels below node
    colors: {
        unchanged: string;
        low: string;
        medium: string;
        high: string;
        background: string;
    };
    animation: {
        pulseDuration: number;  // ms
        fillDuration: number;   // ms
    };
}

export const DEFAULT_HEALTHBAR_CONFIG: HealthbarConfig = {
    width: 0.8,
    height: 4,
    offsetY: 4,
    colors: {
        unchanged: '#666666',
        low: '#4ade80',      // Green
        medium: '#facc15',   // Yellow
        high: '#f87171',     // Red
        background: '#333333',
    },
    animation: {
        pulseDuration: 1000,
        fillDuration: 300,
    },
};

interface HealthbarState {
    currentFill: number;
    targetFill: number;
    animationStart: number | null;
    pulsing: boolean;
}

export class HealthbarRenderer {
    private config: HealthbarConfig;
    private diffs: Map<string, NodeDiffView> = new Map();
    private healthbarStates: Map<string, HealthbarState> = new Map();
    private animationFrameId: number | null = null;

    constructor(
        private network: Network,
        private nodesDataSet: DataSet<Node>,
        config?: Partial<HealthbarConfig>
    ) {
        this.config = { ...DEFAULT_HEALTHBAR_CONFIG, ...config };
        this.setupDrawCallback();
    }

    private setupDrawCallback(): void {
        // vis-network's afterDrawing event for custom canvas drawing
        this.network.on('afterDrawing', (ctx: CanvasRenderingContext2D) => {
            this.drawHealthbars(ctx);
        });
    }

    /**
     * Update diff data for rendering
     */
    updateDiffs(diffs: Map<string, NodeDiffView>): void {
        const now = performance.now();

        for (const [nodeId, diff] of diffs) {
            const existingState = this.healthbarStates.get(nodeId);
            const newFill = this.calculateFillPercent(diff);

            if (!existingState) {
                // New node with diff - animate from 0
                this.healthbarStates.set(nodeId, {
                    currentFill: 0,
                    targetFill: newFill,
                    animationStart: now,
                    pulsing: false,
                });
            } else if (existingState.targetFill !== newFill) {
                // Fill changed - start animation
                existingState.targetFill = newFill;
                existingState.animationStart = now;
            }
        }

        // Remove states for nodes no longer in diffs
        for (const nodeId of this.healthbarStates.keys()) {
            if (!diffs.has(nodeId)) {
                this.healthbarStates.delete(nodeId);
            }
        }

        this.diffs = diffs;
        this.startAnimationLoop();
        this.network.redraw();
    }

    /**
     * Clear all diffs
     */
    clear(): void {
        this.diffs.clear();
        this.healthbarStates.clear();
        this.stopAnimationLoop();
        this.network.redraw();
    }

    /**
     * Set a node to pulse (e.g., when agent is actively changing it)
     */
    setPulsing(nodeId: string, pulsing: boolean): void {
        const state = this.healthbarStates.get(nodeId);
        if (state) {
            state.pulsing = pulsing;
            if (pulsing) {
                this.startAnimationLoop();
            }
        }
        this.network.redraw();
    }

    private calculateFillPercent(diff: NodeDiffView): number {
        if (diff.status === 'unchanged') return 0;
        if (diff.status === 'added' || diff.status === 'removed') return 1;

        // For modified nodes, calculate based on member changes
        const { summary } = diff;
        const totalChanges = summary.membersAdded + summary.membersRemoved + summary.membersModified;

        // Normalize to 0.1 - 1.0 range (always show some fill for modified)
        return Math.max(0.1, Math.min(1, totalChanges / 10));
    }

    private getFillColor(severity: NodeDiffView['severity']): string {
        switch (severity) {
            case 'low':
                return this.config.colors.low;
            case 'medium':
                return this.config.colors.medium;
            case 'high':
                return this.config.colors.high;
            default:
                return this.config.colors.unchanged;
        }
    }

    private drawHealthbars(ctx: CanvasRenderingContext2D): void {
        const now = performance.now();

        for (const [nodeId, diff] of this.diffs) {
            if (diff.status === 'unchanged') continue;

            const state = this.healthbarStates.get(nodeId);
            if (!state) continue;

            // Update animation state
            this.updateAnimationState(state, now);

            // Get node position from network
            const position = this.network.getPosition(nodeId);
            if (!position) continue;

            // Get node bounding box (approximate since vis-network doesn't expose this directly)
            const node = this.nodesDataSet.get(nodeId) as any;
            if (!node) continue;

            // Estimate node width (vis-network default box width is ~label length * font size)
            const nodeWidth = this.estimateNodeWidth(node);
            const nodeHeight = 30; // Approximate default height

            this.drawHealthbar(ctx, position, nodeWidth, nodeHeight, diff, state);
        }
    }

    private drawHealthbar(
        ctx: CanvasRenderingContext2D,
        position: { x: number; y: number },
        nodeWidth: number,
        nodeHeight: number,
        diff: NodeDiffView,
        state: HealthbarState
    ): void {
        const barWidth = nodeWidth * this.config.width;
        const barHeight = this.config.height;
        const x = position.x - barWidth / 2;
        const y = position.y + nodeHeight / 2 + this.config.offsetY;

        // Draw background
        ctx.fillStyle = this.config.colors.background;
        ctx.fillRect(x, y, barWidth, barHeight);

        // Draw fill
        if (state.currentFill > 0) {
            const fillColor = this.getFillColor(diff.severity);

            // Apply pulse effect if active
            if (state.pulsing) {
                const pulsePhase = (performance.now() % this.config.animation.pulseDuration) /
                    this.config.animation.pulseDuration;
                const pulseIntensity = Math.sin(pulsePhase * Math.PI * 2) * 0.3 + 0.7;
                ctx.globalAlpha = pulseIntensity;
            }

            ctx.fillStyle = fillColor;
            ctx.fillRect(x, y, barWidth * state.currentFill, barHeight);
            ctx.globalAlpha = 1;

            // Draw glow effect for high severity
            if (diff.severity === 'high' && state.pulsing) {
                ctx.shadowColor = fillColor;
                ctx.shadowBlur = 8;
                ctx.fillRect(x, y, barWidth * state.currentFill, barHeight);
                ctx.shadowBlur = 0;
            }
        }

        // Draw border
        ctx.strokeStyle = '#555555';
        ctx.lineWidth = 0.5;
        ctx.strokeRect(x, y, barWidth, barHeight);
    }

    private updateAnimationState(state: HealthbarState, now: number): void {
        if (state.animationStart !== null) {
            const elapsed = now - state.animationStart;
            const progress = Math.min(elapsed / this.config.animation.fillDuration, 1);

            // Ease-out cubic
            const eased = 1 - Math.pow(1 - progress, 3);
            const startFill = state.currentFill;
            state.currentFill = startFill + (state.targetFill - startFill) * eased;

            if (progress >= 1) {
                state.animationStart = null;
                state.currentFill = state.targetFill;
            }
        }
    }

    private estimateNodeWidth(node: any): number {
        // Estimate based on label length
        const label = node.label || '';
        const fontSize = node.font?.size || 12;
        // Rough estimate: each character is ~0.6 of font size, plus padding
        return Math.max(60, label.length * fontSize * 0.6 + 20);
    }

    private startAnimationLoop(): void {
        if (this.animationFrameId !== null) return;

        const animate = () => {
            let needsAnimation = false;

            // Check if any nodes need animation
            for (const state of this.healthbarStates.values()) {
                if (state.animationStart !== null || state.pulsing) {
                    needsAnimation = true;
                    break;
                }
            }

            if (needsAnimation) {
                this.network.redraw();
                this.animationFrameId = requestAnimationFrame(animate);
            } else {
                this.animationFrameId = null;
            }
        };

        this.animationFrameId = requestAnimationFrame(animate);
    }

    private stopAnimationLoop(): void {
        if (this.animationFrameId !== null) {
            cancelAnimationFrame(this.animationFrameId);
            this.animationFrameId = null;
        }
    }

    dispose(): void {
        this.stopAnimationLoop();
        this.diffs.clear();
        this.healthbarStates.clear();
    }
}
