/**
 * DiffFileWatcher - File system watcher integration for real-time diff updates
 * Watches for file changes and triggers incremental diff recalculations
 */

import * as vscode from 'vscode';
import type { DiffAnalyzer } from './DiffAnalyzer';
import type { CodeGraph } from '../graph/types';

/**
 * Configuration for the file watcher
 */
export interface DiffFileWatcherConfig {
    // Debounce time in milliseconds
    debounceMs: number;

    // File patterns to watch
    filePatterns: string[];

    // Patterns to exclude
    excludePatterns: string[];
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: DiffFileWatcherConfig = {
    debounceMs: 300,
    filePatterns: [
        '**/*.ts',
        '**/*.tsx',
        '**/*.js',
        '**/*.jsx',
        '**/*.py',
        '**/*.cs',
        '**/*.rs',
        '**/*.go',
    ],
    excludePatterns: [
        '**/node_modules/**',
        '**/dist/**',
        '**/out/**',
        '**/.git/**',
        '**/build/**',
        '**/__pycache__/**',
    ],
};

/**
 * File watcher for diff updates
 */
export class DiffFileWatcher implements vscode.Disposable {
    private watchers: vscode.FileSystemWatcher[] = [];
    private debounceTimers: Map<string, NodeJS.Timeout> = new Map();
    private disposables: vscode.Disposable[] = [];
    private config: DiffFileWatcherConfig;
    private isEnabled: boolean = true;

    // Callback for when files change
    private onFileChangeCallback?: (filePath: string) => Promise<void>;

    constructor(
        private analyzer: DiffAnalyzer,
        private graph: CodeGraph | null,
        private workspaceRoot: string,
        config?: Partial<DiffFileWatcherConfig>
    ) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        this.setupWatchers();
    }

    /**
     * Set up file system watchers
     */
    private setupWatchers(): void {
        // Create a combined pattern
        const pattern = `{${this.config.filePatterns.join(',')}}`;

        const watcher = vscode.workspace.createFileSystemWatcher(
            new vscode.RelativePattern(this.workspaceRoot, pattern)
        );

        // Handle file changes
        this.disposables.push(
            watcher.onDidChange(uri => this.handleFileEvent('change', uri))
        );

        this.disposables.push(
            watcher.onDidCreate(uri => this.handleFileEvent('create', uri))
        );

        this.disposables.push(
            watcher.onDidDelete(uri => this.handleFileEvent('delete', uri))
        );

        this.watchers.push(watcher);
    }

    /**
     * Handle a file system event
     */
    private handleFileEvent(type: 'change' | 'create' | 'delete', uri: vscode.Uri): void {
        if (!this.isEnabled) return;

        const filePath = uri.fsPath;

        // Check exclusion patterns
        if (this.shouldExclude(filePath)) {
            return;
        }

        // Debounce the event
        this.debounce(filePath, async () => {
            await this.processFileChange(filePath, type);
        });
    }

    /**
     * Debounce file change processing
     */
    private debounce(filePath: string, callback: () => Promise<void>): void {
        // Clear existing timer
        const existing = this.debounceTimers.get(filePath);
        if (existing) {
            clearTimeout(existing);
        }

        // Set new timer
        const timer = setTimeout(async () => {
            this.debounceTimers.delete(filePath);
            await callback();
        }, this.config.debounceMs);

        this.debounceTimers.set(filePath, timer);
    }

    /**
     * Process a file change
     */
    private async processFileChange(
        filePath: string,
        type: 'change' | 'create' | 'delete'
    ): Promise<void> {
        try {
            // Invalidate cache
            this.analyzer.invalidate(filePath);

            // Analyze the file
            const diffs = await this.analyzer.analyzeFile(filePath);

            // If we have a graph, recalculate the full graph diff
            if (this.graph) {
                const graphDiff = await this.analyzer.analyzeGraph(this.graph);

                // Fire the diff changed event
                this.analyzer.fireDiffChanged(
                    diffs.map(d => d.nodeId),
                    graphDiff
                );
            }

            // Call custom callback if set
            if (this.onFileChangeCallback) {
                await this.onFileChangeCallback(filePath);
            }
        } catch (error) {
            console.error(`Error processing file change for ${filePath}:`, error);
        }
    }

    /**
     * Check if a file should be excluded
     */
    private shouldExclude(filePath: string): boolean {
        const relativePath = vscode.workspace.asRelativePath(filePath);

        for (const pattern of this.config.excludePatterns) {
            // Simple glob matching
            const regex = this.globToRegex(pattern);
            if (regex.test(relativePath)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Convert glob pattern to regex
     */
    private globToRegex(glob: string): RegExp {
        const escaped = glob
            .replace(/[.+^${}()|[\]\\]/g, '\\$&')
            .replace(/\*\*/g, '{{DOUBLE_STAR}}')
            .replace(/\*/g, '[^/]*')
            .replace(/{{DOUBLE_STAR}}/g, '.*')
            .replace(/\?/g, '.');

        return new RegExp(`^${escaped}$`);
    }

    /**
     * Update the graph reference
     */
    setGraph(graph: CodeGraph): void {
        this.graph = graph;
    }

    /**
     * Set a custom callback for file changes
     */
    onFileChange(callback: (filePath: string) => Promise<void>): void {
        this.onFileChangeCallback = callback;
    }

    /**
     * Enable or disable the watcher
     */
    setEnabled(enabled: boolean): void {
        this.isEnabled = enabled;

        if (!enabled) {
            // Clear pending debounce timers
            for (const timer of this.debounceTimers.values()) {
                clearTimeout(timer);
            }
            this.debounceTimers.clear();
        }
    }

    /**
     * Check if watcher is enabled
     */
    get enabled(): boolean {
        return this.isEnabled;
    }

    /**
     * Manually trigger analysis for a file
     */
    async triggerAnalysis(filePath: string): Promise<void> {
        await this.processFileChange(filePath, 'change');
    }

    /**
     * Manually trigger full graph analysis
     */
    async triggerFullAnalysis(): Promise<void> {
        if (!this.graph) return;

        try {
            // Clear all caches
            this.analyzer.clearCache();

            // Analyze the full graph
            const graphDiff = await this.analyzer.analyzeGraph(this.graph);

            // Get all affected node IDs
            const nodeIds = Array.from(graphDiff.nodeDiffs.keys());

            // Fire event
            this.analyzer.fireDiffChanged(nodeIds, graphDiff);
        } catch (error) {
            console.error('Error during full analysis:', error);
        }
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        // Clear debounce timers
        for (const timer of this.debounceTimers.values()) {
            clearTimeout(timer);
        }
        this.debounceTimers.clear();

        // Dispose watchers
        for (const watcher of this.watchers) {
            watcher.dispose();
        }
        this.watchers = [];

        // Dispose other disposables
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

/**
 * Create and start a diff file watcher
 */
export function createDiffFileWatcher(
    analyzer: DiffAnalyzer,
    workspaceRoot: string,
    graph?: CodeGraph,
    config?: Partial<DiffFileWatcherConfig>
): DiffFileWatcher {
    return new DiffFileWatcher(analyzer, graph || null, workspaceRoot, config);
}
