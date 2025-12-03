/**
 * DiffVisualizationController - Orchestrates all diff visualization components
 * Handles message passing, state management, and coordinates UI components
 */

import type { Network, DataSet, Node, Edge } from 'vis-network/standalone';
import type { NodeDiffView, EdgeDiffView, DiffUpdate } from '../../graph/types';
import { DiffSummaryPopupImpl } from './DiffSummaryPopup';
import { DiffDetailPanelImpl } from './DiffDetailPanel';
import { HealthbarRenderer } from './HealthbarRenderer';

export interface DiffVisualizationConfig {
    enableHealthbars: boolean;
    enablePopup: boolean;
    enablePanel: boolean;
}

const DEFAULT_CONFIG: DiffVisualizationConfig = {
    enableHealthbars: true,
    enablePopup: true,
    enablePanel: true,
};

// Colors for diff status styling
const DIFF_COLORS = {
    added: {
        border: '#4ade80',
        background: 'rgba(74, 222, 128, 0.1)',
    },
    removed: {
        border: '#f87171',
        background: 'rgba(248, 113, 113, 0.1)',
    },
    modified: {
        low: { border: '#4ade80' },
        medium: { border: '#facc15' },
        high: { border: '#f87171' },
    },
};

export class DiffVisualizationController {
    private config: DiffVisualizationConfig;
    private currentDiffs: Map<string, NodeDiffView> = new Map();
    private currentEdgeDiffs: Map<string, EdgeDiffView> = new Map();
    private popup: DiffSummaryPopupImpl | null = null;
    private panel: DiffDetailPanelImpl | null = null;
    private healthbarRenderer: HealthbarRenderer | null = null;
    private baseline: { type: string; reference: string; label?: string } | null = null;

    // Callbacks for extension communication
    public onOpenInEditor?: (nodeId: string) => void;

    constructor(
        private network: Network,
        private nodesDataSet: DataSet<Node>,
        private edgesDataSet: DataSet<Edge>,
        private container: HTMLElement,
        config?: Partial<DiffVisualizationConfig>
    ) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        this.initialize();
    }

    private initialize(): void {
        const graphContainer = document.getElementById('graph-container');

        // Initialize popup
        if (this.config.enablePopup) {
            this.popup = new DiffSummaryPopupImpl(this.container);
        }

        // Initialize panel
        if (this.config.enablePanel) {
            this.panel = new DiffDetailPanelImpl(this.container, graphContainer || undefined);
            this.panel.onOpenInEditor = (nodeId) => {
                this.onOpenInEditor?.(nodeId);
            };
        }

        // Initialize healthbar renderer
        if (this.config.enableHealthbars) {
            this.healthbarRenderer = new HealthbarRenderer(this.network, this.nodesDataSet);
        }

        this.setupEventHandlers();
    }

    private setupEventHandlers(): void {
        // Hover → Show summary popup
        this.network.on('hoverNode', (params: { node: string }) => {
            if (!this.popup) return;

            const diff = this.currentDiffs.get(params.node);
            if (diff && diff.status !== 'unchanged') {
                const position = this.network.getPosition(params.node);
                const canvasPos = this.network.canvasToDOM(position);
                this.popup.show(params.node, canvasPos, diff);
            }
        });

        this.network.on('blurNode', () => {
            this.popup?.hide();
        });

        // Click → Open detail panel
        this.network.on('click', (params: { nodes: string[] }) => {
            if (!this.panel) return;

            if (params.nodes.length === 1) {
                const nodeId = params.nodes[0];
                const diff = this.currentDiffs.get(nodeId);
                if (diff && diff.status !== 'unchanged') {
                    this.panel.open(nodeId, diff);
                } else {
                    this.panel.close();
                }
            } else if (params.nodes.length === 0) {
                // Clicked on empty space
                this.panel.close();
            }
        });
    }

    /**
     * Handle diff update message from extension host
     */
    handleDiffUpdate(payload: DiffUpdate['payload']): void {
        // Update baseline info
        this.baseline = payload.baseline;

        // Store diffs
        this.currentDiffs.clear();
        for (const [nodeId, diff] of Object.entries(payload.nodeDiffs)) {
            this.currentDiffs.set(nodeId, diff);
        }

        this.currentEdgeDiffs.clear();
        for (const [edgeId, diff] of Object.entries(payload.edgeDiffs)) {
            this.currentEdgeDiffs.set(edgeId, diff);
        }

        // Update node styles
        this.updateNodeStyles();

        // Update edge styles
        this.updateEdgeStyles();

        // Handle added nodes (ghost nodes for new code)
        this.handleAddedNodes(payload.addedNodes);

        // Handle removed nodes
        this.handleRemovedNodes(payload.removedNodes);

        // Update healthbars
        if (this.healthbarRenderer) {
            this.healthbarRenderer.updateDiffs(this.currentDiffs);
        }

        // Update open panel if showing a node that was updated
        if (this.panel?.isOpen()) {
            const openNodeId = this.panel.getCurrentNodeId();
            if (openNodeId) {
                const updatedDiff = this.currentDiffs.get(openNodeId);
                if (updatedDiff) {
                    this.panel.update(updatedDiff);
                }
            }
        }
    }

    private updateNodeStyles(): void {
        const nodeUpdates: Node[] = [];

        for (const [nodeId, diff] of this.currentDiffs) {
            const existingNode = this.nodesDataSet.get(nodeId);
            if (!existingNode) continue;

            const newStyle = this.getNodeStyleForDiff(existingNode, diff);
            nodeUpdates.push({
                id: nodeId,
                ...newStyle,
            });
        }

        if (nodeUpdates.length > 0) {
            this.nodesDataSet.update(nodeUpdates);
        }
    }

    private getNodeStyleForDiff(node: Node, diff: NodeDiffView): Partial<Node> {
        if (diff.status === 'unchanged') {
            return {};
        }

        switch (diff.status) {
            case 'added':
                return {
                    borderWidth: 3,
                    color: {
                        border: DIFF_COLORS.added.border,
                        background: (node.color as any)?.background || '#3c3c3c',
                    },
                    shapeProperties: {
                        borderDashes: [5, 5],
                    },
                } as any;

            case 'removed':
                return {
                    opacity: 0.5,
                    borderWidth: 2,
                    color: {
                        border: DIFF_COLORS.removed.border,
                        background: (node.color as any)?.background || '#3c3c3c',
                    },
                    font: {
                        color: '#888888',
                    },
                } as any;

            case 'modified':
                const borderColor = diff.severity === 'high'
                    ? DIFF_COLORS.modified.high.border
                    : diff.severity === 'medium'
                        ? DIFF_COLORS.modified.medium.border
                        : DIFF_COLORS.modified.low.border;

                return {
                    borderWidth: 2,
                    color: {
                        border: borderColor,
                        background: (node.color as any)?.background || '#3c3c3c',
                    },
                } as any;

            default:
                return {};
        }
    }

    private updateEdgeStyles(): void {
        const edgeUpdates: Edge[] = [];

        for (const [edgeId, diff] of this.currentEdgeDiffs) {
            const existingEdge = this.edgesDataSet.get(edgeId);
            if (!existingEdge) continue;

            const newStyle = this.getEdgeStyleForDiff(diff);
            edgeUpdates.push({
                id: edgeId,
                ...newStyle,
            });
        }

        if (edgeUpdates.length > 0) {
            this.edgesDataSet.update(edgeUpdates);
        }
    }

    private getEdgeStyleForDiff(diff: EdgeDiffView): Partial<Edge> {
        if (diff.status === 'unchanged') {
            return {};
        }

        switch (diff.status) {
            case 'added':
                return {
                    color: { color: DIFF_COLORS.added.border, highlight: DIFF_COLORS.added.border },
                    dashes: [5, 5],
                    width: 2,
                } as any;

            case 'removed':
                return {
                    color: { color: DIFF_COLORS.removed.border, opacity: 0.5 },
                    dashes: [2, 4],
                    width: 1,
                } as any;

            default:
                return {};
        }
    }

    private handleAddedNodes(addedNodeIds: string[]): void {
        // For newly added nodes that aren't in the dataset yet,
        // we could add ghost nodes. For now, we just ensure they
        // get the proper styling when they arrive via graph:update
        for (const nodeId of addedNodeIds) {
            const existingNode = this.nodesDataSet.get(nodeId);
            if (existingNode) {
                // Node exists, apply added style
                const diff = this.currentDiffs.get(nodeId);
                if (diff) {
                    const style = this.getNodeStyleForDiff(existingNode, diff);
                    this.nodesDataSet.update({ id: nodeId, ...style });
                }
            }
        }
    }

    private handleRemovedNodes(removedNodeIds: string[]): void {
        for (const nodeId of removedNodeIds) {
            const existingNode = this.nodesDataSet.get(nodeId);
            if (existingNode) {
                // Apply removed style (faded, red border)
                this.nodesDataSet.update({
                    id: nodeId,
                    opacity: 0.3,
                    color: {
                        border: DIFF_COLORS.removed.border,
                        background: '#2a2a2a',
                    },
                    font: { color: '#666666' },
                } as any);
            }
        }
    }

    /**
     * Get current baseline info
     */
    getBaseline(): { type: string; reference: string; label?: string } | null {
        return this.baseline;
    }

    /**
     * Check if a node has changes
     */
    hasChanges(nodeId: string): boolean {
        const diff = this.currentDiffs.get(nodeId);
        return diff !== undefined && diff.status !== 'unchanged';
    }

    /**
     * Get diff for a specific node
     */
    getNodeDiff(nodeId: string): NodeDiffView | undefined {
        return this.currentDiffs.get(nodeId);
    }

    /**
     * Clear all diff visualization
     */
    clear(): void {
        this.currentDiffs.clear();
        this.currentEdgeDiffs.clear();
        this.baseline = null;
        this.healthbarRenderer?.clear();
        this.panel?.close();
    }

    /**
     * Set pulse state for a node (e.g., when agent is actively modifying it)
     */
    setPulsing(nodeId: string, pulsing: boolean): void {
        this.healthbarRenderer?.setPulsing(nodeId, pulsing);
    }

    /**
     * Dispose all resources
     */
    dispose(): void {
        this.popup?.dispose();
        this.panel?.dispose();
        this.healthbarRenderer?.dispose();
        this.currentDiffs.clear();
        this.currentEdgeDiffs.clear();
    }
}
