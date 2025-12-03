import type { SelectionContext, NodeContext, EdgeContext, LayerInfo } from '../../graph/types';

/**
 * Serialize SelectionContext to YAML format
 * Uses a simple YAML serializer without external dependencies
 */
export function toYAML(context: SelectionContext): string {
    const lines: string[] = [];

    lines.push('selection:');
    lines.push(`  summary: "${escapeYAMLString(context.summary)}"`);
    lines.push('');

    // Nodes
    lines.push('  nodes:');
    for (const node of context.nodes) {
        lines.push(...formatNode(node, 4));
    }

    if (context.nodes.length === 0) {
        lines.push('    []');
    }

    lines.push('');

    // Edges
    lines.push('  edges:');
    for (const edge of context.edges) {
        lines.push(...formatEdge(edge, 4));
    }

    if (context.edges.length === 0) {
        lines.push('    []');
    }

    lines.push('');

    // Neighborhood
    lines.push('  visibleNeighborhood:');
    lines.push('    directlyConnected:');

    for (const conn of context.visibleNeighborhood.directlyConnected) {
        lines.push(`      - nodeId: "${conn.nodeId}"`);
        lines.push(`        name: "${escapeYAMLString(conn.name)}"`);
        lines.push(`        type: "${conn.type}"`);
        lines.push(`        signature: "${escapeYAMLString(conn.signature)}"`);
        lines.push(`        connectionType: "${conn.connectionType}"`);
        lines.push(`        edgeType: "${conn.edgeType}"`);
    }

    if (context.visibleNeighborhood.directlyConnected.length === 0) {
        lines.push('      []');
    }

    if (context.visibleNeighborhood.containingCluster) {
        const cluster = context.visibleNeighborhood.containingCluster;
        lines.push('    containingCluster:');
        lines.push(`      name: "${escapeYAMLString(cluster.name)}"`);
        lines.push(`      totalNodes: ${cluster.totalNodes}`);
        lines.push(`      selectedNodes: ${cluster.selectedNodes}`);
    }

    lines.push('');

    // Active Layers
    lines.push('  activeLayers:');
    for (const layer of context.activeLayers) {
        lines.push(...formatLayer(layer, 4));
    }

    lines.push('');

    // Codebase Stats
    lines.push('  codebaseStats:');
    lines.push(`    totalFiles: ${context.codebaseStats.totalFiles}`);
    lines.push(`    totalNodes: ${context.codebaseStats.totalNodes}`);
    lines.push(`    totalEdges: ${context.codebaseStats.totalEdges}`);
    lines.push(`    languages: [${context.codebaseStats.languages.map(l => `"${l}"`).join(', ')}]`);

    return lines.join('\n');
}

function formatNode(node: NodeContext, indent: number): string[] {
    const pad = ' '.repeat(indent);
    const lines: string[] = [];

    lines.push(`${pad}- id: "${node.id}"`);
    lines.push(`${pad}  name: "${escapeYAMLString(node.name)}"`);
    lines.push(`${pad}  type: "${node.type}"`);
    lines.push(`${pad}  language: "${node.language}"`);
    lines.push(`${pad}  file: "${escapeYAMLString(node.file)}"`);
    lines.push(`${pad}  lines: { start: ${node.lines.start}, end: ${node.lines.end} }`);
    lines.push(`${pad}  signature: "${escapeYAMLString(node.signature)}"`);

    // Members list for classes/interfaces (Level 1)
    if (node.members && node.members.length > 0) {
        lines.push(`${pad}  members:`);
        for (const member of node.members) {
            lines.push(`${pad}    - "${escapeYAMLString(member)}"`);
        }
    }

    // Selected lines for partial selection (Level 2)
    if (node.selectedLines) {
        lines.push(`${pad}  selectedLines: { start: ${node.selectedLines.start}, end: ${node.selectedLines.end} }`);
    }

    // Selected code for partial selection (Level 2)
    if (node.selectedCode) {
        lines.push(`${pad}  selectedCode: |`);
        const codeLines = node.selectedCode.split('\n');
        for (const codeLine of codeLines) {
            lines.push(`${pad}    ${codeLine}`);
        }
    }

    // Full code for small nodes (Level 3)
    if (node.code) {
        lines.push(`${pad}  code: |`);
        const codeLines = node.code.split('\n');
        for (const codeLine of codeLines) {
            lines.push(`${pad}    ${codeLine}`);
        }
    }

    lines.push(`${pad}  metrics:`);
    lines.push(`${pad}    loc: ${node.metrics.loc}`);
    if (node.metrics.complexity !== undefined) {
        lines.push(`${pad}    complexity: ${node.metrics.complexity}`);
    }
    if (node.metrics.coverage !== undefined) {
        lines.push(`${pad}    coverage: ${node.metrics.coverage}`);
    }

    lines.push(`${pad}  incomingEdges: ${node.incomingEdges}`);
    lines.push(`${pad}  outgoingEdges: ${node.outgoingEdges}`);

    return lines;
}

function formatEdge(edge: EdgeContext, indent: number): string[] {
    const pad = ' '.repeat(indent);
    const lines: string[] = [];

    lines.push(`${pad}- id: "${edge.id}"`);
    lines.push(`${pad}  from: { nodeId: "${edge.from.nodeId}", name: "${escapeYAMLString(edge.from.name)}", type: "${edge.from.type}" }`);
    lines.push(`${pad}  to: { nodeId: "${edge.to.nodeId}", name: "${escapeYAMLString(edge.to.name)}", type: "${edge.to.type}" }`);
    lines.push(`${pad}  edgeType: "${edge.edgeType}"`);
    lines.push(`${pad}  detail: "${escapeYAMLString(edge.detail)}"`);

    if (edge.location) {
        lines.push(`${pad}  location: { file: "${escapeYAMLString(edge.location.file)}", line: ${edge.location.line} }`);
    }

    return lines;
}

function formatLayer(layer: LayerInfo, indent: number): string[] {
    const pad = ' '.repeat(indent);
    const lines: string[] = [];

    lines.push(`${pad}- name: "${layer.name}"`);
    lines.push(`${pad}  active: ${layer.active}`);

    if (layer.stats) {
        lines.push(`${pad}  stats: { min: ${layer.stats.min.toFixed(1)}, max: ${layer.stats.max.toFixed(1)}, average: ${layer.stats.average.toFixed(1)} }`);
    }

    return lines;
}

/**
 * Escape special characters for YAML strings
 */
function escapeYAMLString(str: string): string {
    return str
        .replace(/\\/g, '\\\\')
        .replace(/"/g, '\\"')
        .replace(/\n/g, '\\n')
        .replace(/\r/g, '\\r')
        .replace(/\t/g, '\\t');
}
