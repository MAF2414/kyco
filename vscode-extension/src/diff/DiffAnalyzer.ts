/**
 * DiffAnalyzer - Main orchestration for diff analysis
 * Calculates structural diffs between baseline and current code
 */

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import type { Tree } from 'tree-sitter';
import type {
    DiffBaseline,
    GraphDiff,
    NodeDiff,
    EdgeDiff,
    DiffChangedEvent,
    DiffAnalyzerConfig,
} from './types';
import { DEFAULT_DIFF_CONFIG } from './types';
import { BaselineResolver } from './BaselineResolver';
import { DiffCache, computeContentHash } from './DiffCache';
import { ASTDiffEngine } from './ASTDiffEngine';
import type { CodeGraph, GraphEdge } from '../graph/types';
import type { LanguageAdapter } from '../graph/LanguageAdapter';
import { getAdapterForFile } from '../graph/adapters';

/**
 * Event emitter for diff changes
 */
class DiffEventEmitter {
    private listeners: ((event: DiffChangedEvent) => void)[] = [];

    on(listener: (event: DiffChangedEvent) => void): vscode.Disposable {
        this.listeners.push(listener);
        return new vscode.Disposable(() => {
            const index = this.listeners.indexOf(listener);
            if (index >= 0) {
                this.listeners.splice(index, 1);
            }
        });
    }

    fire(event: DiffChangedEvent): void {
        for (const listener of this.listeners) {
            try {
                listener(event);
            } catch (error) {
                console.error('Error in diff change listener:', error);
            }
        }
    }
}

/**
 * Parser interface (to be injected)
 */
interface IParser {
    parse(sourceCode: string, language: string): Promise<Tree | null>;
}

/**
 * Main DiffAnalyzer class
 */
export class DiffAnalyzer {
    private baseline: DiffBaseline | null = null;
    private baselineResolver: BaselineResolver;
    private cache: DiffCache;
    private astDiffEngine: ASTDiffEngine;
    private config: DiffAnalyzerConfig;
    private parser: IParser;

    private _onDiffChanged = new DiffEventEmitter();
    public readonly onDiffChanged = this._onDiffChanged;

    constructor(
        private workspaceRoot: string,
        parser: IParser,
        config?: Partial<DiffAnalyzerConfig>
    ) {
        this.parser = parser;
        this.config = { ...DEFAULT_DIFF_CONFIG, ...config };
        this.baselineResolver = new BaselineResolver(workspaceRoot);
        this.cache = new DiffCache(this.config.baselineCacheSize);
        this.astDiffEngine = new ASTDiffEngine();
    }

    /**
     * Set the baseline for all subsequent comparisons
     */
    async setBaseline(baseline: DiffBaseline): Promise<void> {
        this.baseline = baseline;
        this.cache.setBaseline(baseline);
    }

    /**
     * Get the current baseline
     */
    getBaseline(): DiffBaseline | null {
        return this.baseline;
    }

    /**
     * Initialize with default baseline (HEAD~1)
     */
    async initializeDefaultBaseline(): Promise<void> {
        try {
            const defaultBaseline = await this.baselineResolver.git.getDefaultBaseline();
            await this.setBaseline(defaultBaseline);
        } catch (error) {
            console.warn('Could not initialize default baseline:', error);
        }
    }

    /**
     * Analyze the entire graph and compute diffs
     */
    async analyzeGraph(graph: CodeGraph): Promise<GraphDiff> {
        if (!this.baseline) {
            return this.createEmptyGraphDiff();
        }

        const nodeDiffs = new Map<string, NodeDiff>();
        const edgeDiffs = new Map<string, EdgeDiff>();
        const addedNodes: string[] = [];
        const removedNodes: string[] = [];

        // Group nodes by file for efficient processing
        const nodesByFile = this.groupNodesByFile(graph.nodes);

        // Process each file
        for (const [filePath, nodes] of nodesByFile) {
            try {
                const fileDiffs = await this.analyzeFile(filePath);

                for (const diff of fileDiffs) {
                    nodeDiffs.set(diff.nodeId, diff);

                    if (diff.status === 'added') {
                        addedNodes.push(diff.nodeId);
                    }
                }
            } catch (error) {
                console.error(`Error analyzing file ${filePath}:`, error);
            }
        }

        // Check for nodes in baseline that don't exist in current graph
        const currentNodeIds = new Set(graph.nodes.map(n => n.id));
        const baselineFiles = await this.baselineResolver.listFilesInBaseline(this.baseline);

        for (const filePath of baselineFiles) {
            if (this.shouldAnalyzeFile(filePath)) {
                const baselineContent = await this.getBaselineContent(filePath);
                if (baselineContent) {
                    const currentContent = await this.getCurrentContent(filePath);
                    if (!currentContent) {
                        // File was deleted
                        const deletedDiffs = await this.analyzeDeletedFile(filePath, baselineContent);
                        for (const diff of deletedDiffs) {
                            nodeDiffs.set(diff.nodeId, diff);
                            removedNodes.push(diff.nodeId);
                        }
                    }
                }
            }
        }

        // Analyze edges
        for (const edge of graph.edges) {
            const edgeDiff = this.analyzeEdge(edge, nodeDiffs);
            if (edgeDiff.status !== 'unchanged') {
                edgeDiffs.set(edgeDiff.edgeId, edgeDiff);
            }
        }

        // Calculate statistics
        const stats = this.calculateStats(nodeDiffs, edgeDiffs);

        return {
            baseline: this.baseline,
            calculatedAt: new Date(),
            nodeDiffs,
            edgeDiffs,
            addedNodes,
            removedNodes,
            stats,
        };
    }

    /**
     * Analyze a single node
     */
    async analyzeNode(nodeId: string, currentContent: string, filePath: string): Promise<NodeDiff | null> {
        if (!this.baseline) {
            return null;
        }

        const diffs = await this.analyzeFile(filePath, currentContent);
        return diffs.find(d => d.nodeId === nodeId) || null;
    }

    /**
     * Analyze a single file
     */
    async analyzeFile(filePath: string, currentContent?: string): Promise<NodeDiff[]> {
        if (!this.baseline) {
            return [];
        }

        // Get current content if not provided
        if (!currentContent) {
            currentContent = await this.getCurrentContent(filePath) || undefined;
        }

        if (!currentContent) {
            // File doesn't exist - check if it existed in baseline
            const baselineContent = await this.getBaselineContent(filePath);
            if (baselineContent) {
                return this.analyzeDeletedFile(filePath, baselineContent);
            }
            return [];
        }

        // Check cache
        const contentHash = computeContentHash(currentContent);
        const cachedDiffs = this.cache.getFileDiff(filePath, contentHash);
        if (cachedDiffs) {
            return cachedDiffs;
        }

        // Get baseline content
        const baselineContent = await this.getBaselineContent(filePath);

        // Get language adapter
        const adapter = getAdapterForFile(filePath);
        if (!adapter) {
            return [];
        }

        // Parse both versions
        const beforeTree = baselineContent
            ? await this.parser.parse(baselineContent, adapter.languageId)
            : null;
        const afterTree = await this.parser.parse(currentContent, adapter.languageId);

        if (!afterTree && !beforeTree) {
            return [];
        }

        // Run diff
        const diffs = this.astDiffEngine.diffNodes(
            beforeTree,
            afterTree,
            baselineContent || '',
            currentContent,
            adapter
        );

        // Set file path on all diffs
        for (const diff of diffs) {
            diff.filePath = filePath;
        }

        // Cache results
        this.cache.setFileDiff(filePath, contentHash, diffs);

        return diffs;
    }

    /**
     * Invalidate cache for a file
     */
    invalidate(filePath: string): void {
        this.cache.invalidateFile(filePath);
    }

    /**
     * Clear all caches
     */
    clearCache(): void {
        this.cache.clear();
    }

    /**
     * Fire diff changed event
     */
    fireDiffChanged(nodeIds: string[], graphDiff: GraphDiff): void {
        this._onDiffChanged.fire({ nodeIds, graphDiff });
    }

    /**
     * Get baseline content (with caching)
     */
    private async getBaselineContent(filePath: string): Promise<string | null> {
        if (!this.baseline) {
            return null;
        }

        // Check cache first
        const cached = this.cache.getBaselineContent(filePath);
        if (cached !== undefined) {
            return cached;
        }

        // Fetch from resolver
        const content = await this.baselineResolver.getFileContent(filePath, this.baseline);

        // Cache the result (even if null, to avoid repeated lookups)
        if (content !== null) {
            this.cache.setBaselineContent(filePath, content);
        }

        return content;
    }

    /**
     * Get current file content
     */
    private async getCurrentContent(filePath: string): Promise<string | null> {
        const absolutePath = path.isAbsolute(filePath)
            ? filePath
            : path.join(this.workspaceRoot, filePath);

        try {
            return await fs.promises.readFile(absolutePath, 'utf-8');
        } catch {
            return null;
        }
    }

    /**
     * Analyze a file that was deleted
     */
    private async analyzeDeletedFile(filePath: string, baselineContent: string): Promise<NodeDiff[]> {
        const adapter = getAdapterForFile(filePath);
        if (!adapter) {
            return [];
        }

        const beforeTree = await this.parser.parse(baselineContent, adapter.languageId);
        if (!beforeTree) {
            return [];
        }

        const diffs = this.astDiffEngine.diffNodes(
            beforeTree,
            null,
            baselineContent,
            '',
            adapter
        );

        // Set file path on all diffs
        for (const diff of diffs) {
            diff.filePath = filePath;
        }

        return diffs;
    }

    /**
     * Analyze an edge
     */
    private analyzeEdge(edge: GraphEdge, nodeDiffs: Map<string, NodeDiff>): EdgeDiff {
        const fromDiff = nodeDiffs.get(edge.from);
        const toDiff = nodeDiffs.get(edge.to);

        // If both endpoints are unchanged, edge is unchanged
        if (!fromDiff && !toDiff) {
            return {
                edgeId: edge.id,
                fromNodeId: edge.from,
                toNodeId: edge.to,
                status: 'unchanged',
            };
        }

        // If either endpoint is added, edge is added
        if (fromDiff?.status === 'added' || toDiff?.status === 'added') {
            return {
                edgeId: edge.id,
                fromNodeId: edge.from,
                toNodeId: edge.to,
                status: 'added',
            };
        }

        // If either endpoint is removed, edge is removed
        if (fromDiff?.status === 'removed' || toDiff?.status === 'removed') {
            return {
                edgeId: edge.id,
                fromNodeId: edge.from,
                toNodeId: edge.to,
                status: 'removed',
            };
        }

        // Otherwise, check if it's a modified connection
        return {
            edgeId: edge.id,
            fromNodeId: edge.from,
            toNodeId: edge.to,
            status: 'modified',
        };
    }

    /**
     * Check if a file should be analyzed
     */
    private shouldAnalyzeFile(filePath: string): boolean {
        const extensions = ['.ts', '.tsx', '.js', '.jsx', '.py', '.cs', '.rs', '.go'];
        return extensions.some(ext => filePath.endsWith(ext));
    }

    /**
     * Group nodes by file
     */
    private groupNodesByFile(nodes: { filePath: string }[]): Map<string, typeof nodes> {
        const map = new Map<string, typeof nodes>();

        for (const node of nodes) {
            const existing = map.get(node.filePath) || [];
            existing.push(node);
            map.set(node.filePath, existing);
        }

        return map;
    }

    /**
     * Calculate statistics for the graph diff
     */
    private calculateStats(
        nodeDiffs: Map<string, NodeDiff>,
        edgeDiffs: Map<string, EdgeDiff>
    ): GraphDiff['stats'] {
        let totalNodesChanged = 0;
        let low = 0;
        let medium = 0;
        let high = 0;

        for (const diff of nodeDiffs.values()) {
            if (diff.status !== 'unchanged') {
                totalNodesChanged++;
                switch (diff.overallSeverity) {
                    case 'low':
                        low++;
                        break;
                    case 'medium':
                        medium++;
                        break;
                    case 'high':
                        high++;
                        break;
                }
            }
        }

        return {
            totalNodesChanged,
            totalEdgesChanged: edgeDiffs.size,
            bySeverity: { low, medium, high },
        };
    }

    /**
     * Create empty graph diff (when no baseline is set)
     */
    private createEmptyGraphDiff(): GraphDiff {
        return {
            baseline: {
                type: 'working-tree',
                reference: 'HEAD',
                timestamp: new Date(),
            },
            calculatedAt: new Date(),
            nodeDiffs: new Map(),
            edgeDiffs: new Map(),
            addedNodes: [],
            removedNodes: [],
            stats: {
                totalNodesChanged: 0,
                totalEdgesChanged: 0,
                bySeverity: { low: 0, medium: 0, high: 0 },
            },
        };
    }

    // Expose resolvers for advanced use
    get resolver(): BaselineResolver {
        return this.baselineResolver;
    }
}

/**
 * Create a DiffAnalyzer with a simple tree-sitter parser wrapper
 */
export function createDiffAnalyzer(
    workspaceRoot: string,
    parserInstance: { parse: (code: string, lang: string) => Promise<Tree | null> },
    config?: Partial<DiffAnalyzerConfig>
): DiffAnalyzer {
    return new DiffAnalyzer(workspaceRoot, parserInstance, config);
}
