import type { SelectionContext, NodeContext, EdgeContext } from '../../graph/types';

/**
 * Serialize SelectionContext to Markdown format
 * Optimized for chat-based AI agents
 */
export function toMarkdown(context: SelectionContext): string {
    const lines: string[] = [];

    // Summary
    lines.push('## Selection Summary');
    lines.push(context.summary);
    lines.push('');

    // Selected Nodes
    if (context.nodes.length > 0) {
        lines.push('### Selected Nodes');
        lines.push('');

        for (const node of context.nodes) {
            lines.push(...formatNodeMarkdown(node));
            lines.push('');
        }
    }

    // Selected Edges
    if (context.edges.length > 0) {
        lines.push('### Selected Connections');
        lines.push('');

        let index = 1;
        for (const edge of context.edges) {
            lines.push(...formatEdgeMarkdown(edge, index++));
        }
        lines.push('');
    }

    // Neighborhood
    if (context.visibleNeighborhood.directlyConnected.length > 0) {
        lines.push('### Neighborhood');
        lines.push('');
        lines.push('Directly connected but not selected:');

        for (const conn of context.visibleNeighborhood.directlyConnected) {
            lines.push(`- **${conn.name}** (${conn.type}) - \`${conn.signature}\` [${conn.connectionType}, ${conn.edgeType}]`);
        }
        lines.push('');

        if (context.visibleNeighborhood.containingCluster) {
            const cluster = context.visibleNeighborhood.containingCluster;
            lines.push(`Containing cluster: **${cluster.name}** (${cluster.selectedNodes}/${cluster.totalNodes} nodes selected)`);
            lines.push('');
        }
    }

    // Active Layers with stats
    const activeLayers = context.activeLayers.filter(l => l.active && l.stats);
    if (activeLayers.length > 0) {
        lines.push('### Active Layers');

        for (const layer of activeLayers) {
            if (layer.stats) {
                const unit = layer.name === 'coverage' ? '%' : '';
                lines.push(`- **${capitalize(layer.name)}**: min ${layer.stats.min.toFixed(1)}${unit}, max ${layer.stats.max.toFixed(1)}${unit}, avg ${layer.stats.average.toFixed(1)}${unit}`);
            }
        }
        lines.push('');
    }

    // Codebase Stats
    lines.push('### Codebase Overview');
    lines.push(`- Total files: ${context.codebaseStats.totalFiles}`);
    lines.push(`- Total nodes: ${context.codebaseStats.totalNodes}`);
    lines.push(`- Total edges: ${context.codebaseStats.totalEdges}`);
    lines.push(`- Languages: ${context.codebaseStats.languages.join(', ')}`);

    return lines.join('\n');
}

function formatNodeMarkdown(node: NodeContext): string[] {
    const lines: string[] = [];

    lines.push(`#### ${node.name} (${node.type})`);
    lines.push(`- **File:** ${node.file}:${node.lines.start}-${node.lines.end}`);
    lines.push(`- **Signature:** \`${node.signature}\``);

    // Members list for classes/interfaces (Level 1)
    if (node.members && node.members.length > 0) {
        lines.push(`- **Members:**`);
        for (const member of node.members) {
            lines.push(`  - \`${member}\``);
        }
    }

    // Metrics line
    const metricsParts: string[] = [`${node.metrics.loc} LOC`];
    if (node.metrics.complexity !== undefined) {
        metricsParts.push(`complexity ${node.metrics.complexity}`);
    }
    if (node.metrics.coverage !== undefined) {
        metricsParts.push(`coverage ${node.metrics.coverage.toFixed(1)}%`);
    }
    lines.push(`- **Metrics:** ${metricsParts.join(', ')}`);

    lines.push(`- **Connections:** ${node.incomingEdges} incoming, ${node.outgoingEdges} outgoing`);

    // Selected code for partial selection (Level 2)
    if (node.selectedLines && node.selectedCode) {
        lines.push('');
        lines.push(`**Selected lines ${node.selectedLines.start}-${node.selectedLines.end}:**`);
        lines.push('```' + getLanguageIdentifier(node.language));
        lines.push(node.selectedCode);
        lines.push('```');
    }
    // Full code for small nodes (Level 3)
    else if (node.code) {
        lines.push('');
        lines.push('```' + getLanguageIdentifier(node.language));
        lines.push(node.code);
        lines.push('```');
    }

    return lines;
}

function formatEdgeMarkdown(edge: EdgeContext, index: number): string[] {
    const lines: string[] = [];

    lines.push(`${index}. **${edge.from.name}** -> **${edge.to.name}** (${edge.edgeType})`);
    lines.push(`   - \`${edge.detail}\``);

    if (edge.location) {
        lines.push(`   - Location: ${edge.location.file}:${edge.location.line}`);
    }

    return lines;
}

function getLanguageIdentifier(language: string): string {
    const mapping: Record<string, string> = {
        'typescript': 'typescript',
        'javascript': 'javascript',
        'python': 'python',
        'csharp': 'csharp',
        'rust': 'rust',
        'go': 'go',
    };
    return mapping[language.toLowerCase()] || language;
}

function capitalize(str: string): string {
    return str.charAt(0).toUpperCase() + str.slice(1);
}
