/**
 * Core types for the Code Map graph representation
 */

export type NodeType = 'file' | 'directory' | 'namespace' | 'class' | 'interface' | 'function' | 'method' | 'export';

export type EdgeType = 'import' | 'call' | 'inheritance' | 'implementation' | 'contains';

export interface GraphNode {
    id: string;
    label: string;
    type: NodeType;
    language: string;
    filePath: string;
    lineStart: number;
    lineEnd: number;
    isExported: boolean;
    parentId: string | null;
    signature?: string;
    metrics: NodeMetrics;
}

export interface NodeMetrics {
    loc: number;
    coverage?: number;
    complexity?: number;
}

export interface GraphEdge {
    id: string;
    from: string;
    to: string;
    type: EdgeType;
    label?: string;
}

export interface CodeGraph {
    nodes: GraphNode[];
    edges: GraphEdge[];
    metadata: GraphMetadata;
}

export interface GraphMetadata {
    generatedAt: number;
    workspaceRoot: string;
    languages: string[];
    nodeCount: number;
    edgeCount: number;
}

/**
 * vis-network compatible format
 */
export interface VisNode {
    id: string;
    label: string;
    title?: string;
    group?: string;
    level?: number;
    color?: string | { background: string; border: string };
    size?: number;
    borderWidth?: number;
    shape?: string;
    font?: { size: number; color: string };
    // Custom data
    data: GraphNode;
}

export interface VisEdge {
    id: string;
    from: string;
    to: string;
    label?: string;
    arrows?: string;
    dashes?: boolean;
    color?: string | { color: string; opacity: number };
    width?: number;
    // Custom data
    data: GraphEdge;
}

export interface VisGraph {
    nodes: VisNode[];
    edges: VisEdge[];
}

/**
 * Agent context for selection
 */
export interface AgentContextNode {
    nodeId: string;
    type: NodeType;
    name: string;
    language: string;
    filePath: string;
    lineStart: number;
    lineEnd: number;
    code: string;
    signature?: string;
    metrics: NodeMetrics;
}

export interface AgentContext {
    selection: AgentContextNode[];
    prompt?: string;
}

/**
 * Messages between Extension Host and Webview
 */
export type ExtensionToWebviewMessage =
    | { type: 'graph:update'; payload: VisGraph }
    | { type: 'graph:patch'; payload: { nodes?: VisNode[]; edges?: VisEdge[]; removeNodeIds?: string[]; removeEdgeIds?: string[] } }
    | { type: 'metrics:update'; payload: { nodeId: string; metrics: NodeMetrics }[] }
    | { type: 'config:update'; payload: WebviewConfig }
    | DiffUpdate;

export type WebviewToExtensionMessage =
    | { type: 'node:open'; payload: { nodeId: string; filePath: string; line: number } }
    | { type: 'selection:send'; payload: { nodeIds: string[]; edgeIds: string[]; locRanges?: { file: string; start: number; end: number }[]; viewport: { x: number; y: number; scale: number }; prompt?: string } }
    | { type: 'graph:refresh' }
    | { type: 'ready' };

export interface WebviewConfig {
    layout: 'hierarchical' | 'force-directed';
    activeLayer: 'coverage' | 'loc' | 'complexity' | 'none';
}

/**
 * Coverage data structures
 */
export interface FileCoverage {
    filePath: string;
    linesCovered: number;
    linesTotal: number;
    percentage: number;
    functions: FunctionCoverage[];
}

export interface FunctionCoverage {
    name: string;
    lineStart: number;
    lineEnd: number;
    hitCount: number;
    percentage: number;
}

export interface CoverageReport {
    files: Map<string, FileCoverage>;
    totalCoverage: number;
}

/**
 * Selection types for SelectionSerializer
 */

// Input: What the user selected in the map
export interface MapSelection {
    nodeIds: string[];
    edgeIds: string[];

    // Explicit LOC-ranges (e.g. via Shift+Drag in the map)
    locRanges?: {
        file: string;
        start: number;
        end: number;
    }[];

    viewport: {
        x: number;
        y: number;
        scale: number;
    };
}

// Output: Rich context for the agent
export interface SelectionContext {
    summary: string;
    nodes: NodeContext[];
    edges: EdgeContext[];
    visibleNeighborhood: NeighborhoodContext;
    activeLayers: LayerInfo[];
    codebaseStats: CodebaseStats;
}

export interface NodeContext {
    id: string;
    name: string;
    type: NodeType;
    language: string;
    file: string;
    lines: { start: number; end: number };
    signature: string;

    // Level 1: For classes/interfaces - member signatures (methods, properties)
    members?: string[];

    // Level 2: User explicitly selected a LOC-range
    selectedLines?: { start: number; end: number };
    selectedCode?: string;

    // Level 3: Small nodes - full code (automatic when under threshold)
    code?: string;

    metrics: {
        loc: number;
        complexity?: number;
        coverage?: number;
    };

    incomingEdges: number;
    outgoingEdges: number;
}

export interface EdgeContext {
    id: string;
    from: {
        nodeId: string;
        name: string;
        type: string;
    };
    to: {
        nodeId: string;
        name: string;
        type: string;
    };
    edgeType: 'imports' | 'calls' | 'extends' | 'implements' | 'type-reference' | 'contains';
    detail: string;
    location?: {
        file: string;
        line: number;
    };
}

export interface NeighborReference {
    nodeId: string;
    name: string;
    type: string;
    signature: string;
    connectionType: 'incoming' | 'outgoing';
    edgeType: string;
}

export interface NeighborhoodContext {
    directlyConnected: NeighborReference[];
    containingCluster?: {
        name: string;
        totalNodes: number;
        selectedNodes: number;
    };
}

export interface LayerInfo {
    name: 'coverage' | 'loc' | 'complexity';
    active: boolean;
    stats?: {
        min: number;
        max: number;
        average: number;
    };
}

export interface CodebaseStats {
    totalFiles: number;
    totalNodes: number;
    totalEdges: number;
    languages: string[];
}

export type OutputFormat = 'yaml' | 'json' | 'markdown';

export interface AgentPayload {
    context: string;
    prompt: string;
    format: OutputFormat;
}

/**
 * Diff visualization types for webview
 */
export interface DiffUpdate {
    type: 'diff:update';
    payload: {
        nodeDiffs: Record<string, NodeDiffView>;
        edgeDiffs: Record<string, EdgeDiffView>;
        addedNodes: string[];
        removedNodes: string[];
        baseline: {
            type: string;
            reference: string;
            label?: string;
        };
    };
}

export interface NodeDiffView {
    nodeId: string;
    status: 'unchanged' | 'modified' | 'added' | 'removed';
    severity: 'none' | 'low' | 'medium' | 'high';
    summary: {
        membersAdded: number;
        membersRemoved: number;
        membersModified: number;
        linesAdded: number;
        linesRemoved: number;
    };
    memberDiffs: MemberDiffView[];
}

export interface MemberDiffView {
    name: string;
    type: 'method' | 'property' | 'constructor' | 'getter' | 'setter';
    changeType: 'unchanged' | 'added' | 'removed' | 'modified';
    severity: 'none' | 'low' | 'medium' | 'high';
    signatureChange?: {
        before: string;
        after: string;
    };
    linesAdded: number;
    linesRemoved: number;
}

export interface EdgeDiffView {
    edgeId: string;
    status: 'unchanged' | 'added' | 'removed' | 'modified';
}

// Code inclusion levels for SelectionSerializer
export type CodeInclusionLevel = 'signature' | 'partial' | 'full';

export interface SelectionSerializerConfig {
    // Threshold under which full code is included
    fullCodeLocThreshold: number;           // Default: 25

    // Maximum lines for selectedCode
    maxSelectedCodeLines: number;           // Default: 100

    // Maximum members listed
    maxMembersListed: number;               // Default: 20

    // Neighborhood depth
    neighborhoodDepth: 1 | 2;               // Default: 1 (only direct connections)
}
