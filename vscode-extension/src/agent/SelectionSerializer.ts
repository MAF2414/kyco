import * as fs from 'fs';
import type {
    MapSelection,
    SelectionContext,
    NodeContext,
    EdgeContext,
    NeighborhoodContext,
    NeighborReference,
    LayerInfo,
    CodebaseStats,
    VisGraph,
    VisNode,
    VisEdge,
    NodeType,
    CodeInclusionLevel,
    SelectionSerializerConfig,
} from '../graph/types';
import { toYAML } from './formatters/yaml';
import { toJSON } from './formatters/json';
import { toMarkdown } from './formatters/markdown';

const DEFAULT_CONFIG: SelectionSerializerConfig = {
    fullCodeLocThreshold: 25,
    maxSelectedCodeLines: 100,
    maxMembersListed: 20,
    neighborhoodDepth: 1,
};

/**
 * Maps internal edge types to the output format edge types
 */
function mapEdgeType(type: string): EdgeContext['edgeType'] {
    const mapping: Record<string, EdgeContext['edgeType']> = {
        'import': 'imports',
        'call': 'calls',
        'inheritance': 'extends',
        'implementation': 'implements',
        'contains': 'contains',
    };
    return mapping[type] || 'type-reference';
}

/**
 * SelectionSerializer transforms visual map selection into structured context for AI agents.
 * It extracts code, metadata, relationships, and metrics from selected nodes and edges.
 *
 * Code inclusion levels:
 * - Level 1 (signature): For large nodes - only signature + members list
 * - Level 2 (partial): For explicit LOC-range selection - selected lines only
 * - Level 3 (full): For small nodes - complete code automatically included
 */
export class SelectionSerializer {
    private config: SelectionSerializerConfig;

    constructor(config?: Partial<SelectionSerializerConfig>) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }

    /**
     * Serialize the map selection into a rich SelectionContext
     */
    async serialize(
        selection: MapSelection,
        graph: VisGraph,
        activeLayers: string[],
        workspaceRoot: string
    ): Promise<SelectionContext> {
        // Build lookup maps
        const nodeMap = new Map<string, VisNode>();
        const edgeMap = new Map<string, VisEdge>();

        for (const node of graph.nodes) {
            nodeMap.set(node.id, node);
        }
        for (const edge of graph.edges) {
            edgeMap.set(edge.id, edge);
        }

        // Build edge index for quick connection lookup
        const incomingEdges = new Map<string, VisEdge[]>();
        const outgoingEdges = new Map<string, VisEdge[]>();

        for (const edge of graph.edges) {
            const from = edge.from;
            const to = edge.to;

            if (!outgoingEdges.has(from)) outgoingEdges.set(from, []);
            if (!incomingEdges.has(to)) incomingEdges.set(to, []);

            outgoingEdges.get(from)!.push(edge);
            incomingEdges.get(to)!.push(edge);
        }

        // Extract node contexts
        const selectedNodeSet = new Set(selection.nodeIds);
        const nodes: NodeContext[] = [];

        for (const nodeId of selection.nodeIds) {
            const visNode = nodeMap.get(nodeId);
            if (!visNode?.data || visNode.data.type === 'directory') continue;

            const nodeContext = await this.extractNodeContext(
                visNode,
                selection,
                incomingEdges.get(nodeId)?.length || 0,
                outgoingEdges.get(nodeId)?.length || 0,
                workspaceRoot
            );
            nodes.push(nodeContext);
        }

        // Extract edge contexts
        const edges: EdgeContext[] = [];

        for (const edgeId of selection.edgeIds) {
            const visEdge = edgeMap.get(edgeId);
            if (!visEdge) continue;

            const fromNode = nodeMap.get(visEdge.from);
            const toNode = nodeMap.get(visEdge.to);
            if (!fromNode || !toNode) continue;

            const edgeContext = await this.extractEdgeContext(visEdge, fromNode, toNode, workspaceRoot);
            edges.push(edgeContext);
        }

        // Build neighborhood context
        const visibleNeighborhood = this.buildNeighborhoodContext(
            selection.nodeIds,
            selectedNodeSet,
            nodeMap,
            incomingEdges,
            outgoingEdges
        );

        // Build layer info with statistics
        const layerInfos = this.buildLayerInfo(activeLayers, nodes);

        // Build codebase stats
        const codebaseStats = this.buildCodebaseStats(graph);

        // Generate summary
        const summary = this.generateSummary(nodes, edges);

        return {
            summary,
            nodes,
            edges,
            visibleNeighborhood,
            activeLayers: layerInfos,
            codebaseStats,
        };
    }

    /**
     * Determine the code inclusion level for a node
     */
    private determineCodeInclusion(
        node: VisNode['data'],
        selection: MapSelection
    ): CodeInclusionLevel {
        // Check 1: Does the user have an explicit LOC-range within this node?
        const explicitRange = selection.locRanges?.find(range =>
            range.file === node.filePath &&
            range.start >= node.lineStart &&
            range.end <= node.lineEnd
        );

        if (explicitRange) {
            return 'partial';
        }

        // Check 2: Is the node small enough for full code inclusion?
        if (node.metrics.loc <= this.config.fullCodeLocThreshold) {
            return 'full';
        }

        // Default: Only signature + members
        return 'signature';
    }

    /**
     * Extract detailed context for a single node
     */
    private async extractNodeContext(
        visNode: VisNode,
        selection: MapSelection,
        incomingCount: number,
        outgoingCount: number,
        workspaceRoot: string
    ): Promise<NodeContext> {
        const data = visNode.data;
        const inclusionLevel = this.determineCodeInclusion(data, selection);

        // Read file content for code extraction
        let fileContent = '';
        try {
            fileContent = await fs.promises.readFile(data.filePath, 'utf-8');
        } catch {
            // File not readable
        }

        const lines = fileContent.split('\n');

        // Extract signature
        const fullCode = lines.slice(data.lineStart - 1, data.lineEnd).join('\n');
        const signature = data.signature || this.extractSignature(fullCode, data.type) || data.label;

        // Make file path relative to workspace
        const relativeFile = data.filePath.startsWith(workspaceRoot)
            ? data.filePath.slice(workspaceRoot.length + 1)
            : data.filePath;

        const base: NodeContext = {
            id: data.id,
            name: data.label,
            type: data.type,
            language: data.language,
            file: relativeFile,
            lines: { start: data.lineStart, end: data.lineEnd },
            signature,
            metrics: {
                loc: data.metrics.loc,
                complexity: data.metrics.complexity,
                coverage: data.metrics.coverage,
            },
            incomingEdges: incomingCount,
            outgoingEdges: outgoingCount,
        };

        // Members for classes/interfaces (Level 1)
        if (data.type === 'class' || data.type === 'interface') {
            base.members = this.extractMembers(fullCode, data.type, data.language);
        }

        // Code inclusion based on level
        switch (inclusionLevel) {
            case 'full':
                base.code = fullCode;
                break;

            case 'partial': {
                const range = selection.locRanges!.find(r =>
                    r.file === data.filePath &&
                    r.start >= data.lineStart
                )!;
                const selectedStart = Math.max(range.start, data.lineStart);
                const selectedEnd = Math.min(range.end, data.lineEnd);

                // Limit to maxSelectedCodeLines
                const lineCount = selectedEnd - selectedStart + 1;
                const actualEnd = lineCount > this.config.maxSelectedCodeLines
                    ? selectedStart + this.config.maxSelectedCodeLines - 1
                    : selectedEnd;

                base.selectedLines = { start: selectedStart, end: actualEnd };
                base.selectedCode = lines.slice(selectedStart - 1, actualEnd).join('\n');
                break;
            }

            case 'signature':
                // Only signature + members, no code
                break;
        }

        return base;
    }

    /**
     * Extract member signatures from class/interface code
     */
    private extractMembers(code: string, type: NodeType, language: string): string[] {
        const members: string[] = [];
        const lines = code.split('\n');

        // Track brace depth to find top-level members
        let braceDepth = 0;
        let foundBody = false;

        for (const line of lines) {
            const trimmed = line.trim();

            // Track brace depth
            for (const char of trimmed) {
                if (char === '{') {
                    if (!foundBody) foundBody = true;
                    braceDepth++;
                } else if (char === '}') {
                    braceDepth--;
                }
            }

            // Only look at lines at depth 1 (inside class/interface body)
            if (!foundBody || braceDepth !== 1) continue;

            // Skip comments and empty lines
            if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('*')) {
                continue;
            }

            // Extract member signature based on language
            const memberSig = this.extractMemberSignature(trimmed, language);
            if (memberSig) {
                members.push(memberSig);
            }

            // Limit members
            if (members.length >= this.config.maxMembersListed) {
                break;
            }
        }

        return members;
    }

    /**
     * Extract a single member signature from a line of code
     */
    private extractMemberSignature(line: string, language: string): string | null {
        // TypeScript/JavaScript patterns
        if (language === 'typescript' || language === 'javascript') {
            // Constructor
            if (line.startsWith('constructor')) {
                return line.replace(/\{.*$/, '').trim();
            }

            // Method (with or without modifiers)
            const methodMatch = line.match(/^(public|private|protected|static|async|readonly|\s)*(\w+)\s*(\(|<)/);
            if (methodMatch && !line.includes('=')) {
                return line.replace(/\{.*$/, '').trim();
            }

            // Property
            const propMatch = line.match(/^(public|private|protected|static|readonly|\s)*(\w+)\s*[?:]?\s*:/);
            if (propMatch) {
                return line.replace(/;.*$/, '').trim();
            }
        }

        // Python patterns
        if (language === 'python') {
            if (line.startsWith('def ') || line.startsWith('async def ')) {
                return line.replace(/:.*$/, '').trim();
            }
        }

        // C# patterns
        if (language === 'csharp') {
            const csMethodMatch = line.match(/^(public|private|protected|internal|static|virtual|override|async|\s)*[\w<>\[\]]+\s+\w+\s*\(/);
            if (csMethodMatch) {
                return line.replace(/\{.*$/, '').trim();
            }
        }

        return null;
    }

    /**
     * Extract detailed context for a single edge
     */
    private async extractEdgeContext(
        visEdge: VisEdge,
        fromNode: VisNode,
        toNode: VisNode,
        workspaceRoot: string
    ): Promise<EdgeContext> {
        const edgeData = visEdge.data;

        // Try to find the actual import/call statement location
        const location = await this.findEdgeLocation(fromNode, toNode, edgeData.type, workspaceRoot);

        // Build detail string based on edge type
        const detail = this.buildEdgeDetail(edgeData, fromNode.data.label, toNode.data.label);

        return {
            id: edgeData.id,
            from: {
                nodeId: fromNode.data.id,
                name: fromNode.data.label,
                type: fromNode.data.type,
            },
            to: {
                nodeId: toNode.data.id,
                name: toNode.data.label,
                type: toNode.data.type,
            },
            edgeType: mapEdgeType(edgeData.type),
            detail,
            location,
        };
    }

    /**
     * Build neighborhood context for nodes connected to selection but not selected
     */
    private buildNeighborhoodContext(
        selectedNodeIds: string[],
        selectedNodeSet: Set<string>,
        nodeMap: Map<string, VisNode>,
        incomingEdges: Map<string, VisEdge[]>,
        outgoingEdges: Map<string, VisEdge[]>
    ): NeighborhoodContext {
        const directlyConnected: NeighborReference[] = [];
        const seenConnections = new Set<string>();

        for (const nodeId of selectedNodeIds) {
            // Check outgoing edges
            const outEdges = outgoingEdges.get(nodeId) || [];
            for (const edge of outEdges) {
                const targetId = edge.to;
                if (!selectedNodeSet.has(targetId) && !seenConnections.has(targetId)) {
                    seenConnections.add(targetId);
                    const targetNode = nodeMap.get(targetId);
                    if (targetNode?.data) {
                        directlyConnected.push({
                            nodeId: targetId,
                            name: targetNode.data.label,
                            type: targetNode.data.type,
                            signature: targetNode.data.signature || this.getNodeTypePrefix(targetNode.data.type) + targetNode.data.label,
                            connectionType: 'outgoing',
                            edgeType: edge.data?.type || 'unknown',
                        });
                    }
                }
            }

            // Check incoming edges
            const inEdges = incomingEdges.get(nodeId) || [];
            for (const edge of inEdges) {
                const sourceId = edge.from;
                if (!selectedNodeSet.has(sourceId) && !seenConnections.has(sourceId)) {
                    seenConnections.add(sourceId);
                    const sourceNode = nodeMap.get(sourceId);
                    if (sourceNode?.data) {
                        directlyConnected.push({
                            nodeId: sourceId,
                            name: sourceNode.data.label,
                            type: sourceNode.data.type,
                            signature: sourceNode.data.signature || this.getNodeTypePrefix(sourceNode.data.type) + sourceNode.data.label,
                            connectionType: 'incoming',
                            edgeType: edge.data?.type || 'unknown',
                        });
                    }
                }
            }
        }

        // Find containing cluster (parent directory/namespace)
        const containingCluster = this.findContainingCluster(selectedNodeIds, nodeMap);

        return {
            directlyConnected,
            containingCluster,
        };
    }

    /**
     * Get a type prefix for generating default signatures
     */
    private getNodeTypePrefix(type: NodeType): string {
        switch (type) {
            case 'class': return 'class ';
            case 'interface': return 'interface ';
            case 'function': return 'function ';
            case 'method': return '';
            case 'namespace': return 'namespace ';
            default: return '';
        }
    }

    /**
     * Find the common parent cluster for selected nodes
     */
    private findContainingCluster(
        selectedNodeIds: string[],
        nodeMap: Map<string, VisNode>
    ): NeighborhoodContext['containingCluster'] | undefined {
        if (selectedNodeIds.length === 0) return undefined;

        // Get parent IDs of all selected nodes
        const parentIds = new Set<string>();

        for (const nodeId of selectedNodeIds) {
            const node = nodeMap.get(nodeId);
            if (node?.data?.parentId) {
                parentIds.add(node.data.parentId);
            }
        }

        // If all nodes share the same parent
        if (parentIds.size === 1) {
            const parentId = Array.from(parentIds)[0];
            const parentNode = nodeMap.get(parentId);
            if (parentNode?.data) {
                // Count total nodes with this parent
                let totalNodes = 0;
                nodeMap.forEach((n) => {
                    if (n.data?.parentId === parentId) totalNodes++;
                });

                return {
                    name: parentNode.data.label,
                    totalNodes,
                    selectedNodes: selectedNodeIds.length,
                };
            }
        }

        return undefined;
    }

    /**
     * Build layer information with statistics
     */
    private buildLayerInfo(activeLayers: string[], nodes: NodeContext[]): LayerInfo[] {
        const layers: LayerInfo[] = [];

        const layerNames: Array<'coverage' | 'loc' | 'complexity'> = ['coverage', 'loc', 'complexity'];

        for (const name of layerNames) {
            const active = activeLayers.includes(name);
            let stats: LayerInfo['stats'] | undefined;

            if (active && nodes.length > 0) {
                const values: number[] = [];

                for (const node of nodes) {
                    let value: number | undefined;
                    if (name === 'coverage') value = node.metrics.coverage;
                    else if (name === 'loc') value = node.metrics.loc;
                    else if (name === 'complexity') value = node.metrics.complexity;

                    if (value !== undefined) values.push(value);
                }

                if (values.length > 0) {
                    stats = {
                        min: Math.min(...values),
                        max: Math.max(...values),
                        average: values.reduce((a, b) => a + b, 0) / values.length,
                    };
                }
            }

            layers.push({ name, active, stats });
        }

        return layers;
    }

    /**
     * Build codebase statistics
     */
    private buildCodebaseStats(graph: VisGraph): CodebaseStats {
        const languages = new Set<string>();
        const files = new Set<string>();

        for (const node of graph.nodes) {
            if (node.data?.language) languages.add(node.data.language);
            if (node.data?.filePath) files.add(node.data.filePath);
        }

        return {
            totalFiles: files.size,
            totalNodes: graph.nodes.length,
            totalEdges: graph.edges.length,
            languages: Array.from(languages),
        };
    }

    /**
     * Generate human-readable summary
     */
    private generateSummary(nodes: NodeContext[], edges: EdgeContext[]): string {
        const typeCounts = new Map<string, number>();
        const files = new Set<string>();

        for (const node of nodes) {
            typeCounts.set(node.type, (typeCounts.get(node.type) || 0) + 1);
            files.add(node.file);
        }

        const parts: string[] = [];

        const typeOrder: NodeType[] = ['file', 'class', 'interface', 'function', 'method', 'namespace'];
        for (const type of typeOrder) {
            const count = typeCounts.get(type);
            if (count) {
                const plural = count > 1 ? this.pluralize(type) : type;
                parts.push(`${count} ${plural}`);
            }
        }

        // Check for remaining types
        for (const [type, count] of typeCounts) {
            if (!typeOrder.includes(type as NodeType)) {
                const plural = count > 1 ? this.pluralize(type) : type;
                parts.push(`${count} ${plural}`);
            }
        }

        const nodeDescription = parts.length > 0 ? parts.join(', ') : 'no items';
        const edgeInfo = edges.length > 0
            ? ` with ${edges.length} selected connection${edges.length > 1 ? 's' : ''}`
            : '';
        const fileInfo = ` in ${files.size} file${files.size !== 1 ? 's' : ''}`;

        return `Selection contains ${nodeDescription}${edgeInfo}${fileInfo}`;
    }

    /**
     * Simple pluralization
     */
    private pluralize(word: string): string {
        if (word.endsWith('s')) return word + 'es';
        if (word.endsWith('y')) return word.slice(0, -1) + 'ies';
        return word + 's';
    }

    /**
     * Extract code from file at specified line range
     */
    private async extractCode(filePath: string, lineStart: number, lineEnd: number): Promise<string> {
        try {
            const content = await fs.promises.readFile(filePath, 'utf-8');
            const lines = content.split('\n');

            const startIndex = Math.max(0, lineStart - 1);
            const endIndex = Math.min(lines.length, lineEnd);

            return lines.slice(startIndex, endIndex).join('\n');
        } catch {
            return '';
        }
    }

    /**
     * Extract signature from code based on node type
     */
    private extractSignature(code: string, type: NodeType): string | undefined {
        if (!code) return undefined;

        const lines = code.split('\n');
        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('*')) {
                continue;
            }

            // Return first meaningful line as signature
            if (type === 'class' && (trimmed.includes('class ') || trimmed.includes('export class '))) {
                return trimmed.replace(/\{.*$/, '').trim();
            }
            if (type === 'interface' && (trimmed.includes('interface ') || trimmed.includes('export interface '))) {
                return trimmed.replace(/\{.*$/, '').trim();
            }
            if (type === 'function' || type === 'method') {
                if (trimmed.includes('function ') || trimmed.includes('async ') ||
                    trimmed.match(/^\w+\s*\(/) || trimmed.match(/^(export\s+)?(const|let|var)\s+\w+\s*=/)) {
                    return trimmed.replace(/\{.*$/, '').trim();
                }
            }
            // For Python
            if (trimmed.startsWith('def ') || trimmed.startsWith('async def ') || trimmed.startsWith('class ')) {
                return trimmed.replace(/:.*$/, '').trim();
            }

            return trimmed;
        }

        return undefined;
    }

    /**
     * Find the location in code where an edge relationship is defined
     */
    private async findEdgeLocation(
        fromNode: VisNode,
        toNode: VisNode,
        edgeType: string,
        workspaceRoot: string
    ): Promise<{ file: string; line: number } | undefined> {
        try {
            const content = await fs.promises.readFile(fromNode.data.filePath, 'utf-8');
            const lines = content.split('\n');

            const targetName = toNode.data.label;
            const relativeFile = fromNode.data.filePath.startsWith(workspaceRoot)
                ? fromNode.data.filePath.slice(workspaceRoot.length + 1)
                : fromNode.data.filePath;

            for (let i = 0; i < lines.length; i++) {
                const line = lines[i];

                if (edgeType === 'import' && line.includes('import') && line.includes(targetName)) {
                    return { file: relativeFile, line: i + 1 };
                }

                if (edgeType === 'inheritance' && (line.includes('extends') || line.includes(':')) && line.includes(targetName)) {
                    return { file: relativeFile, line: i + 1 };
                }

                if (edgeType === 'implementation' && line.includes('implements') && line.includes(targetName)) {
                    return { file: relativeFile, line: i + 1 };
                }
            }

            return undefined;
        } catch {
            return undefined;
        }
    }

    /**
     * Build a detail string describing the edge relationship
     */
    private buildEdgeDetail(edgeData: VisEdge['data'], fromName: string, toName: string): string {
        switch (edgeData.type) {
            case 'import':
                return `import { ${toName} } from '...'`;
            case 'call':
                return `${fromName} calls ${toName}()`;
            case 'inheritance':
                return `${fromName} extends ${toName}`;
            case 'implementation':
                return `${fromName} implements ${toName}`;
            case 'contains':
                return `${fromName} contains ${toName}`;
            default:
                return edgeData.label || `${fromName} -> ${toName}`;
        }
    }

    /**
     * Serialize context to YAML format
     */
    toYAML(context: SelectionContext): string {
        return toYAML(context);
    }

    /**
     * Serialize context to JSON format
     */
    toJSON(context: SelectionContext): string {
        return toJSON(context);
    }

    /**
     * Serialize context to Markdown format
     */
    toMarkdown(context: SelectionContext): string {
        return toMarkdown(context);
    }
}
