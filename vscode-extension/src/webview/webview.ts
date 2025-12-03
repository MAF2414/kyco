/**
 * Webview script for Code Map visualization using vis-network
 */

import { Network, DataSet, Options, Node, Edge } from 'vis-network/standalone';
import type {
    VisNode,
    VisEdge,
    VisGraph,
    WebviewConfig,
    ExtensionToWebviewMessage,
    WebviewToExtensionMessage,
    NodeMetrics,
    DiffUpdate,
} from '../graph/types';
import { DiffVisualizationController } from './diff/DiffVisualizationController';

// Declare VS Code API
declare function acquireVsCodeApi(): {
    postMessage(message: WebviewToExtensionMessage): void;
    getState(): any;
    setState(state: any): void;
};

const vscode = acquireVsCodeApi();

// State
let network: Network | null = null;
let nodesDataSet: DataSet<Node> = new DataSet();
let edgesDataSet: DataSet<Edge> = new DataSet();
let selectedNodes: Set<string> = new Set();
let selectedEdges: Set<string> = new Set();
let locRanges: { file: string; start: number; end: number }[] = [];
let currentConfig: WebviewConfig = {
    layout: 'hierarchical',
    activeLayer: 'none',
};
let originalNodeColors: Map<string, any> = new Map();
let clusterThreshold = 0.7; // Zoom level below which clustering activates
const DEEP_ZOOM_THRESHOLD = 2.0; // Zoom level above which full code is included

// DOM Elements
let graphContainer: HTMLElement;
let selectionCountEl: HTMLElement;
let statusNodesEl: HTMLElement;
let statusEdgesEl: HTMLElement;
let statusZoomEl: HTMLElement;
let coverageLegend: HTMLElement;

// Diff visualization
let diffController: DiffVisualizationController | null = null;
let baselineSection: HTMLElement;
let baselineTypeEl: HTMLElement;
let baselineRefEl: HTMLElement;

/**
 * Initialize the webview
 */
function init(): void {
    graphContainer = document.getElementById('graph-container')!;
    selectionCountEl = document.getElementById('selection-count')!;
    statusNodesEl = document.getElementById('status-nodes')!;
    statusEdgesEl = document.getElementById('status-edges')!;
    statusZoomEl = document.getElementById('status-zoom')!;
    coverageLegend = document.getElementById('coverage-legend')!;
    baselineSection = document.getElementById('baseline-section')!;
    baselineTypeEl = document.getElementById('baseline-type')!;
    baselineRefEl = document.getElementById('baseline-ref')!;

    // Initialize network with empty data
    initNetwork();

    // Initialize diff visualization controller
    initDiffVisualization();

    // Set up event listeners
    setupToolbarListeners();
    setupKeyboardShortcuts();

    // Notify extension that webview is ready
    vscode.postMessage({ type: 'ready' });
}

/**
 * Initialize vis-network
 */
function initNetwork(): void {
    const options = getNetworkOptions(currentConfig.layout);

    network = new Network(graphContainer, { nodes: nodesDataSet, edges: edgesDataSet }, options);

    // Event: Node selection
    network.on('selectNode', (params) => {
        const nodeIds = params.nodes as string[];
        handleNodeSelection(nodeIds, params.event);
    });

    network.on('deselectNode', (params) => {
        if (!params.event?.srcEvent?.ctrlKey && !params.event?.srcEvent?.metaKey) {
            selectedNodes.clear();
        }
        updateSelectionUI();
    });

    // Event: Edge selection
    network.on('selectEdge', (params) => {
        const edgeIds = params.edges as string[];
        handleEdgeSelection(edgeIds, params.event);
    });

    network.on('deselectEdge', (params) => {
        if (!params.event?.srcEvent?.ctrlKey && !params.event?.srcEvent?.metaKey) {
            selectedEdges.clear();
        }
        updateSelectionUI();
    });

    // Event: Double-click to open file
    network.on('doubleClick', (params) => {
        if (params.nodes.length > 0) {
            const nodeId = params.nodes[0] as string;
            openNode(nodeId);
        }
    });

    // Event: Zoom change for clustering and LOC-range selection
    network.on('zoom', (params) => {
        const scale = params.scale;
        statusZoomEl.textContent = `Zoom: ${Math.round(scale * 100)}%`;
        handleZoomClustering(scale);
        handleDeepZoomLocRanges(scale);
    });

    // Event: Right-click context menu
    network.on('oncontext', (params) => {
        params.event.preventDefault();
        if (params.nodes.length > 0) {
            showContextMenu(params.event, params.nodes[0] as string);
        }
    });

    // Hide context menu on click elsewhere
    document.addEventListener('click', hideContextMenu);
}

/**
 * Get network options based on layout type
 */
function getNetworkOptions(layout: 'hierarchical' | 'force-directed'): Options {
    const baseOptions: Options = {
        nodes: {
            shape: 'box',
            font: {
                size: 12,
                color: '#cccccc',
            },
            borderWidth: 2,
            shadow: true,
        },
        edges: {
            smooth: {
                type: 'cubicBezier',
                forceDirection: layout === 'hierarchical' ? 'vertical' : 'none',
            },
            arrows: {
                to: { enabled: true, scaleFactor: 0.5 },
            },
            font: {
                size: 10,
                color: '#808080',
            },
        },
        physics: {
            enabled: layout === 'force-directed',
            solver: 'forceAtlas2Based',
            forceAtlas2Based: {
                gravitationalConstant: -50,
                centralGravity: 0.01,
                springLength: 100,
                springConstant: 0.08,
            },
            stabilization: {
                enabled: true,
                iterations: 200,
            },
        },
        interaction: {
            multiselect: true,
            selectConnectedEdges: false,
            hover: true,
            tooltipDelay: 200,
        },
        layout: layout === 'hierarchical' ? {
            hierarchical: {
                enabled: true,
                direction: 'UD',
                sortMethod: 'directed',
                levelSeparation: 100,
                nodeSpacing: 150,
            },
        } : {
            hierarchical: false,
        },
    };

    return baseOptions;
}

/**
 * Initialize diff visualization controller
 */
function initDiffVisualization(): void {
    if (!network) return;

    diffController = new DiffVisualizationController(
        network,
        nodesDataSet,
        edgesDataSet,
        document.body
    );

    // Set up callback for opening nodes in editor
    diffController.onOpenInEditor = (nodeId: string) => {
        openNode(nodeId);
    };
}

/**
 * Handle diff update from extension
 */
function handleDiffUpdate(payload: DiffUpdate['payload']): void {
    if (!diffController) return;

    // Update diff visualization
    diffController.handleDiffUpdate(payload);

    // Update baseline badge in toolbar
    if (payload.baseline) {
        baselineSection.style.display = 'flex';
        baselineTypeEl.textContent = payload.baseline.type;
        baselineRefEl.textContent = payload.baseline.label || payload.baseline.reference;
    } else {
        baselineSection.style.display = 'none';
    }
}

/**
 * Handle messages from extension
 */
window.addEventListener('message', (event) => {
    const message = event.data as ExtensionToWebviewMessage;

    switch (message.type) {
        case 'graph:update':
            updateGraph(message.payload);
            break;
        case 'graph:patch':
            patchGraph(message.payload);
            break;
        case 'metrics:update':
            updateMetrics(message.payload);
            break;
        case 'config:update':
            updateConfig(message.payload);
            break;
        case 'diff:update':
            handleDiffUpdate(message.payload);
            break;
    }
});

/**
 * Update the entire graph
 */
function updateGraph(graph: VisGraph): void {
    // Store original colors before applying layers
    originalNodeColors.clear();
    for (const node of graph.nodes) {
        originalNodeColors.set(node.id, node.color);
    }

    // Convert to vis-network format
    const nodes = graph.nodes.map(n => ({
        ...n,
        title: n.title || buildTooltip(n),
    }));

    const edges = graph.edges.map(e => ({
        ...e,
        hidden: e.data?.type === 'contains', // Hide contains edges initially
    }));

    // Update datasets
    nodesDataSet.clear();
    edgesDataSet.clear();
    nodesDataSet.add(nodes as any);
    edgesDataSet.add(edges as any);

    // Update status
    statusNodesEl.textContent = `Nodes: ${nodes.length}`;
    statusEdgesEl.textContent = `Edges: ${edges.length}`;

    // Apply current layer
    applyMetricsLayer(currentConfig.activeLayer);

    // Fit view after short delay
    setTimeout(() => {
        network?.fit({ animation: true });
    }, 100);
}

/**
 * Patch graph with incremental updates
 */
function patchGraph(patch: {
    nodes?: VisNode[];
    edges?: VisEdge[];
    removeNodeIds?: string[];
    removeEdgeIds?: string[];
}): void {
    if (patch.removeNodeIds) {
        nodesDataSet.remove(patch.removeNodeIds);
    }
    if (patch.removeEdgeIds) {
        edgesDataSet.remove(patch.removeEdgeIds);
    }
    if (patch.nodes) {
        nodesDataSet.update(patch.nodes as any);
    }
    if (patch.edges) {
        edgesDataSet.update(patch.edges as any);
    }

    statusNodesEl.textContent = `Nodes: ${nodesDataSet.length}`;
    statusEdgesEl.textContent = `Edges: ${edgesDataSet.length}`;
}

/**
 * Update metrics on nodes
 */
function updateMetrics(updates: { nodeId: string; metrics: NodeMetrics }[]): void {
    for (const update of updates) {
        const node = nodesDataSet.get(update.nodeId);
        if (node) {
            (node as any).data.metrics = { ...(node as any).data.metrics, ...update.metrics };
            nodesDataSet.update(node);
        }
    }

    // Re-apply current layer
    applyMetricsLayer(currentConfig.activeLayer);
}

/**
 * Update configuration
 */
function updateConfig(config: WebviewConfig): void {
    currentConfig = config;

    // Update layout if changed
    if (network) {
        const options = getNetworkOptions(config.layout);
        network.setOptions(options);
    }

    // Update active layer
    applyMetricsLayer(config.activeLayer);
}

/**
 * Apply metrics layer to nodes
 */
function applyMetricsLayer(layer: 'coverage' | 'loc' | 'complexity' | 'none'): void {
    const updates: any[] = [];

    nodesDataSet.forEach((node: any) => {
        const data = node.data;
        if (!data) return;

        let color = originalNodeColors.get(node.id);
        let size = node.size || 20;

        switch (layer) {
            case 'coverage':
                if (data.metrics?.coverage !== undefined) {
                    color = getCoverageColor(data.metrics.coverage);
                }
                break;
            case 'loc':
                size = Math.max(15, Math.min(60, Math.sqrt(data.metrics?.loc || 10) * 4));
                break;
            case 'complexity':
                if (data.metrics?.complexity !== undefined) {
                    const intensity = Math.min(1, data.metrics.complexity / 20);
                    color = getComplexityColor(intensity);
                }
                break;
            case 'none':
            default:
                color = originalNodeColors.get(node.id);
                break;
        }

        updates.push({
            id: node.id,
            color,
            size,
        });
    });

    nodesDataSet.update(updates);

    // Show/hide coverage legend
    if (layer === 'coverage') {
        coverageLegend.classList.remove('hidden');
    } else {
        coverageLegend.classList.add('hidden');
    }
}

/**
 * Get color based on coverage percentage
 */
function getCoverageColor(coverage: number): { background: string; border: string } {
    // Red (0%) -> Yellow (50%) -> Green (100%)
    let r, g, b;

    if (coverage < 50) {
        // Red to Yellow
        const t = coverage / 50;
        r = 217;
        g = Math.round(83 + (173 - 83) * t);
        b = Math.round(79 + (78 - 79) * t);
    } else {
        // Yellow to Green
        const t = (coverage - 50) / 50;
        r = Math.round(240 - (240 - 92) * t);
        g = Math.round(173 + (184 - 173) * t);
        b = Math.round(78 + (92 - 78) * t);
    }

    const bg = `rgb(${r}, ${g}, ${b})`;
    const border = `rgb(${Math.round(r * 0.7)}, ${Math.round(g * 0.7)}, ${Math.round(b * 0.7)})`;

    return { background: bg, border };
}

/**
 * Get color based on complexity intensity
 */
function getComplexityColor(intensity: number): { background: string; border: string } {
    // Light blue (low) -> Dark red (high)
    const r = Math.round(100 + 155 * intensity);
    const g = Math.round(200 - 150 * intensity);
    const b = Math.round(255 - 200 * intensity);

    const bg = `rgb(${r}, ${g}, ${b})`;
    const border = `rgb(${Math.round(r * 0.7)}, ${Math.round(g * 0.7)}, ${Math.round(b * 0.7)})`;

    return { background: bg, border };
}

/**
 * Handle zoom-based clustering
 */
function handleZoomClustering(scale: number): void {
    if (!network) return;

    if (scale < clusterThreshold) {
        // Cluster by directory/parent
        clusterByParent();
    } else {
        // Open all clusters
        openAllClusters();
    }
}

/**
 * Handle deep zoom for automatic LOC-range selection
 * When zoomed in deeply, include full code for selected nodes
 */
function handleDeepZoomLocRanges(scale: number): void {
    if (scale > DEEP_ZOOM_THRESHOLD && selectedNodes.size > 0) {
        // At deep zoom: include full LOC-ranges for selected nodes
        locRanges = [];
        selectedNodes.forEach(nodeId => {
            const node = nodesDataSet.get(nodeId) as any;
            if (node?.data) {
                locRanges.push({
                    file: node.data.filePath,
                    start: node.data.lineStart,
                    end: node.data.lineEnd,
                });
            }
        });
    } else {
        // Not deep zoom: clear LOC-ranges
        locRanges = [];
    }
}

/**
 * Cluster nodes by their parent
 */
function clusterByParent(): void {
    if (!network) return;

    // Get all directory nodes
    const dirNodes = nodesDataSet.get({
        filter: (node: any) => node.data?.type === 'directory',
    });

    for (const dirNode of dirNodes) {
        const clusterId = `cluster_${dirNode.id}`;

        // Check if already clustered
        if (network.isCluster(clusterId)) continue;

        const clusterOptions = {
            joinCondition: (nodeOptions: any) => {
                return nodeOptions.data?.parentId === dirNode.id;
            },
            clusterNodeProperties: {
                id: clusterId,
                label: `${(dirNode as any).label} (...)`,
                shape: 'box',
                color: { background: '#666666', border: '#888888' },
                font: { color: '#ffffff' },
            },
        };

        try {
            network.cluster(clusterOptions);
        } catch {
            // Clustering may fail if no nodes match
        }
    }
}

/**
 * Open all clusters
 */
function openAllClusters(): void {
    if (!network) return;

    // Get all cluster node IDs
    const allNodes = nodesDataSet.getIds();
    for (const nodeId of allNodes) {
        if (typeof nodeId === 'string' && nodeId.startsWith('cluster_')) {
            try {
                network.openCluster(nodeId);
            } catch {
                // May already be open
            }
        }
    }
}

/**
 * Handle node selection
 */
function handleNodeSelection(nodeIds: string[], event: any): void {
    const ctrlKey = event?.srcEvent?.ctrlKey || event?.srcEvent?.metaKey;
    const shiftKey = event?.srcEvent?.shiftKey;

    if (ctrlKey) {
        // Toggle selection
        for (const id of nodeIds) {
            if (selectedNodes.has(id)) {
                selectedNodes.delete(id);
            } else {
                selectedNodes.add(id);
            }
        }
    } else if (shiftKey && selectedNodes.size > 0) {
        // Add to selection (range selection would need position info)
        for (const id of nodeIds) {
            selectedNodes.add(id);
        }
    } else {
        // Single selection - clear both nodes and edges unless ctrl is held
        selectedNodes.clear();
        selectedEdges.clear();
        for (const id of nodeIds) {
            selectedNodes.add(id);
        }
    }

    updateSelectionUI();
}

/**
 * Handle edge selection
 */
function handleEdgeSelection(edgeIds: string[], event: any): void {
    const ctrlKey = event?.srcEvent?.ctrlKey || event?.srcEvent?.metaKey;
    const shiftKey = event?.srcEvent?.shiftKey;

    if (ctrlKey) {
        // Toggle selection
        for (const id of edgeIds) {
            if (selectedEdges.has(id)) {
                selectedEdges.delete(id);
            } else {
                selectedEdges.add(id);
            }
        }
    } else if (shiftKey && (selectedNodes.size > 0 || selectedEdges.size > 0)) {
        // Add to selection
        for (const id of edgeIds) {
            selectedEdges.add(id);
        }
    } else {
        // Single selection - clear both unless ctrl is held
        selectedNodes.clear();
        selectedEdges.clear();
        for (const id of edgeIds) {
            selectedEdges.add(id);
        }
    }

    updateSelectionUI();
}

/**
 * Update selection UI
 */
function updateSelectionUI(): void {
    const nodeCount = selectedNodes.size;
    const edgeCount = selectedEdges.size;
    const totalCount = nodeCount + edgeCount;

    // Build selection count text
    const parts: string[] = [];
    if (nodeCount > 0) {
        parts.push(`${nodeCount} node${nodeCount > 1 ? 's' : ''}`);
    }
    if (edgeCount > 0) {
        parts.push(`${edgeCount} edge${edgeCount > 1 ? 's' : ''}`);
    }
    selectionCountEl.textContent = parts.length > 0 ? parts.join(', ') + ' selected' : '0 selected';

    // Update network selection
    if (network) {
        network.selectNodes(Array.from(selectedNodes));
        network.selectEdges(Array.from(selectedEdges));
    }

    // Enable/disable send button
    const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;
    if (sendBtn) {
        sendBtn.disabled = totalCount === 0;
    }
}

/**
 * Open node in editor
 */
function openNode(nodeId: string): void {
    const node = nodesDataSet.get(nodeId) as any;
    if (!node?.data) return;

    vscode.postMessage({
        type: 'node:open',
        payload: {
            nodeId,
            filePath: node.data.filePath,
            line: node.data.lineStart,
        },
    });
}

/**
 * Send selection to agent
 */
function sendSelectionToAgent(): void {
    if (selectedNodes.size === 0 && selectedEdges.size === 0) return;

    // Get current viewport position
    const viewPosition = network?.getViewPosition() || { x: 0, y: 0 };
    const scale = network?.getScale() || 1;

    // Update LOC ranges based on current zoom level
    handleDeepZoomLocRanges(scale);

    vscode.postMessage({
        type: 'selection:send',
        payload: {
            nodeIds: Array.from(selectedNodes),
            edgeIds: Array.from(selectedEdges),
            locRanges: locRanges.length > 0 ? locRanges : undefined,
            viewport: {
                x: viewPosition.x,
                y: viewPosition.y,
                scale: scale,
            },
        },
    });
}

/**
 * Request graph refresh
 */
function refreshGraph(): void {
    vscode.postMessage({ type: 'graph:refresh' });
}

/**
 * Build tooltip HTML for a node
 */
function buildTooltip(node: VisNode): string {
    const data = node.data;
    const lines: string[] = [
        `<b>${data.label}</b>`,
        `Type: ${data.type}`,
        `Language: ${data.language || 'N/A'}`,
    ];

    if (data.lineStart && data.lineEnd) {
        lines.push(`Lines: ${data.lineStart}-${data.lineEnd}`);
    }

    lines.push(`LOC: ${data.metrics?.loc || 0}`);

    if (data.signature) {
        lines.push(`<br><code>${data.signature}</code>`);
    }

    if (data.metrics?.coverage !== undefined) {
        lines.push(`Coverage: ${data.metrics.coverage.toFixed(1)}%`);
    }

    if (data.metrics?.complexity !== undefined) {
        lines.push(`Complexity: ${data.metrics.complexity}`);
    }

    return lines.join('<br>');
}

/**
 * Show context menu
 */
function showContextMenu(event: MouseEvent, nodeId: string): void {
    let menu = document.getElementById('context-menu');

    if (!menu) {
        menu = document.createElement('div');
        menu.id = 'context-menu';
        document.body.appendChild(menu);
    }

    const node = nodesDataSet.get(nodeId) as any;

    menu.innerHTML = `
        <div class="context-menu-item" data-action="open">Open in Editor</div>
        <div class="context-menu-item" data-action="select">Add to Selection</div>
        <div class="context-menu-separator"></div>
        <div class="context-menu-item" data-action="send">Send to Agent</div>
        <div class="context-menu-separator"></div>
        <div class="context-menu-item" data-action="expand">Expand Children</div>
        <div class="context-menu-item" data-action="collapse">Collapse Children</div>
    `;

    menu.style.left = `${event.clientX}px`;
    menu.style.top = `${event.clientY}px`;
    menu.classList.remove('hidden');

    // Handle menu item clicks
    menu.querySelectorAll('.context-menu-item').forEach((item) => {
        item.addEventListener('click', () => {
            const action = (item as HTMLElement).dataset.action;
            handleContextMenuAction(action!, nodeId);
            hideContextMenu();
        });
    });
}

/**
 * Hide context menu
 */
function hideContextMenu(): void {
    const menu = document.getElementById('context-menu');
    if (menu) {
        menu.classList.add('hidden');
    }
}

/**
 * Handle context menu action
 */
function handleContextMenuAction(action: string, nodeId: string): void {
    switch (action) {
        case 'open':
            openNode(nodeId);
            break;
        case 'select':
            selectedNodes.add(nodeId);
            updateSelectionUI();
            break;
        case 'send':
            selectedNodes.add(nodeId);
            sendSelectionToAgent();
            break;
        case 'expand':
            expandNode(nodeId);
            break;
        case 'collapse':
            collapseNode(nodeId);
            break;
    }
}

/**
 * Expand node to show children
 */
function expandNode(nodeId: string): void {
    if (!network) return;

    const clusterId = `cluster_${nodeId}`;
    if (network.isCluster(clusterId)) {
        network.openCluster(clusterId);
    }
}

/**
 * Collapse node to hide children
 */
function collapseNode(nodeId: string): void {
    if (!network) return;

    const clusterId = `cluster_${nodeId}`;
    if (!network.isCluster(clusterId)) {
        const clusterOptions = {
            joinCondition: (nodeOptions: any) => {
                return nodeOptions.data?.parentId === nodeId;
            },
            clusterNodeProperties: {
                id: clusterId,
                label: `${(nodesDataSet.get(nodeId) as any)?.label || nodeId} (...)`,
                shape: 'box',
                color: { background: '#666666', border: '#888888' },
            },
        };

        try {
            network.cluster(clusterOptions);
        } catch {
            // May fail if no children
        }
    }
}

/**
 * Set up toolbar event listeners
 */
function setupToolbarListeners(): void {
    // Layer buttons
    document.querySelectorAll('.layer-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.layer-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            const layer = (btn as HTMLElement).dataset.layer as WebviewConfig['activeLayer'];
            currentConfig.activeLayer = layer;
            applyMetricsLayer(layer);
        });
    });

    // Layout buttons
    document.querySelectorAll('.layout-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.layout-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            const layout = (btn as HTMLElement).dataset.layout as WebviewConfig['layout'];
            currentConfig.layout = layout;

            if (network) {
                const options = getNetworkOptions(layout);
                network.setOptions(options);

                // Re-stabilize for force-directed
                if (layout === 'force-directed') {
                    network.stabilize(200);
                }
            }
        });
    });

    // Refresh button
    document.getElementById('refresh-btn')?.addEventListener('click', refreshGraph);

    // Send button
    document.getElementById('send-btn')?.addEventListener('click', sendSelectionToAgent);
}

/**
 * Set up keyboard shortcuts
 */
function setupKeyboardShortcuts(): void {
    document.addEventListener('keydown', (e) => {
        // Ctrl+Enter: Send selection to agent
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
            e.preventDefault();
            sendSelectionToAgent();
        }

        // Escape: Clear selection
        if (e.key === 'Escape') {
            selectedNodes.clear();
            selectedEdges.clear();
            updateSelectionUI();
        }

        // Ctrl+A: Select all visible nodes
        if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
            e.preventDefault();
            nodesDataSet.forEach((node: any) => {
                selectedNodes.add(node.id);
            });
            updateSelectionUI();
        }

        // F5: Refresh
        if (e.key === 'F5') {
            e.preventDefault();
            refreshGraph();
        }

        // +/-: Zoom
        if (e.key === '+' || e.key === '=') {
            network?.moveTo({ scale: network.getScale() * 1.2 });
        }
        if (e.key === '-') {
            network?.moveTo({ scale: network.getScale() / 1.2 });
        }

        // 0: Reset zoom
        if (e.key === '0') {
            network?.fit({ animation: true });
        }
    });
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
