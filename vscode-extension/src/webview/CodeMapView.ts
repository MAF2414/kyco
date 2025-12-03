import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { GraphBuilder } from '../graph/GraphBuilder';
import { MetricsCollector } from '../graph/MetricsCollector';
import { AgentContextBuilder } from '../agent/AgentContextBuilder';
import { DiffAnalyzer, createDiffAnalyzer, DiffFileWatcher, createDiffFileWatcher } from '../diff';
import type { GraphDiff, NodeDiff, EdgeDiff } from '../diff';
import type {
    VisGraph,
    WebviewConfig,
    ExtensionToWebviewMessage,
    WebviewToExtensionMessage,
    MapSelection,
    OutputFormat,
    NodeDiffView,
    EdgeDiffView,
    DiffUpdate,
} from '../graph/types';

/**
 * Manages the Code Map webview panel
 */
export class CodeMapViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'kyco.codeMapView';

    private _view?: vscode.WebviewView;
    private _panel?: vscode.WebviewPanel;
    private graphBuilder: GraphBuilder;
    private metricsCollector: MetricsCollector;
    private agentContextBuilder: AgentContextBuilder;
    private currentGraph?: VisGraph;
    private disposables: vscode.Disposable[] = [];

    private workspaceRoot: string = '';
    private activeLayer: string = 'none';

    // Diff analysis
    private diffAnalyzer?: DiffAnalyzer;
    private diffFileWatcher?: DiffFileWatcher;

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly extensionContext: vscode.ExtensionContext
    ) {
        this.graphBuilder = new GraphBuilder();
        this.metricsCollector = new MetricsCollector();
        this.agentContextBuilder = new AgentContextBuilder();
    }

    /**
     * Called when the webview view is resolved
     */
    public resolveWebviewView(
        webviewView: vscode.WebviewView,
        _context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken
    ): void {
        this._view = webviewView;

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(this.extensionUri, 'out'),
                vscode.Uri.joinPath(this.extensionUri, 'src'),
            ],
        };

        webviewView.webview.html = this.getHtmlForWebview(webviewView.webview);

        this.setupMessageHandler(webviewView.webview);
    }

    /**
     * Open Code Map in a dedicated panel
     */
    public async openPanel(): Promise<void> {
        if (this._panel) {
            this._panel.reveal(vscode.ViewColumn.Beside);
            return;
        }

        this._panel = vscode.window.createWebviewPanel(
            'kycoCodeMap',
            'Code Map',
            vscode.ViewColumn.Beside,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
                localResourceRoots: [
                    vscode.Uri.joinPath(this.extensionUri, 'out'),
                    vscode.Uri.joinPath(this.extensionUri, 'src'),
                ],
            }
        );

        this._panel.webview.html = this.getHtmlForWebview(this._panel.webview);

        this.setupMessageHandler(this._panel.webview);

        // Set context for keybindings
        vscode.commands.executeCommand('setContext', 'kyco.codeMapFocused', true);

        this._panel.onDidChangeViewState(e => {
            vscode.commands.executeCommand(
                'setContext',
                'kyco.codeMapFocused',
                e.webviewPanel.active
            );
        });

        this._panel.onDidDispose(() => {
            this._panel = undefined;
            vscode.commands.executeCommand('setContext', 'kyco.codeMapFocused', false);
        });
    }

    /**
     * Refresh the graph
     */
    public async refresh(): Promise<void> {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showErrorMessage('No workspace folder open');
            return;
        }

        this.workspaceRoot = workspaceFolders[0].uri.fsPath;
        const workspaceRoot = this.workspaceRoot;

        // Get configuration
        const config = vscode.workspace.getConfiguration('kyco.codeMap');
        const includePatterns = config.get<string[]>('include', [
            '**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx',
            '**/*.py', '**/*.cs', '**/*.rs', '**/*.go'
        ]);
        const excludePatterns = config.get<string[]>('exclude', [
            '**/node_modules/**', '**/dist/**', '**/out/**',
            '**/.git/**', '**/build/**', '**/__pycache__/**'
        ]);
        const coverageFile = config.get<string>('coverageFile', '');

        // Show progress
        await vscode.window.withProgress(
            {
                location: vscode.ProgressLocation.Notification,
                title: 'Building Code Map...',
                cancellable: false,
            },
            async (progress) => {
                try {
                    // Build graph
                    progress.report({ message: 'Parsing files...' });
                    const graph = await this.graphBuilder.buildGraph({
                        workspaceRoot,
                        includePatterns,
                        excludePatterns,
                    });

                    // Load coverage if configured
                    if (coverageFile) {
                        progress.report({ message: 'Loading coverage...' });
                        const coveragePath = path.isAbsolute(coverageFile)
                            ? coverageFile
                            : path.join(workspaceRoot, coverageFile);

                        await this.metricsCollector.loadCoverage(coveragePath);
                    }

                    // Calculate complexity
                    progress.report({ message: 'Calculating metrics...' });
                    const complexityMaps = new Map<string, Map<string, number>>();

                    for (const node of graph.nodes) {
                        if (node.type === 'file' && !complexityMaps.has(node.filePath)) {
                            try {
                                const sourceCode = await fs.promises.readFile(node.filePath, 'utf-8');
                                const complexity = await this.metricsCollector.calculateComplexity(
                                    node.filePath,
                                    sourceCode
                                );
                                complexityMaps.set(node.filePath, complexity);
                            } catch {
                                // Skip files that can't be read
                            }
                        }
                    }

                    // Apply metrics
                    this.metricsCollector.applyMetrics(graph.nodes, workspaceRoot, complexityMaps);

                    // Convert to vis-network format
                    progress.report({ message: 'Rendering graph...' });
                    this.currentGraph = this.graphBuilder.toVisGraph(graph);

                    // Update AgentContextBuilder with graph and workspace root
                    this.agentContextBuilder.setGraph(this.currentGraph);
                    this.agentContextBuilder.setWorkspaceRoot(workspaceRoot);
                    this.agentContextBuilder.setActiveLayers(
                        this.activeLayer !== 'none' ? [this.activeLayer] : []
                    );

                    // Send to webview
                    this.postMessage({
                        type: 'graph:update',
                        payload: this.currentGraph,
                    });

                    // Initialize diff analysis
                    progress.report({ message: 'Initializing diff analysis...' });
                    await this.initializeDiffAnalysis(graph);

                } catch (error) {
                    vscode.window.showErrorMessage(`Failed to build code map: ${error}`);
                }
            }
        );
    }

    /**
     * Initialize diff analysis for the current graph
     */
    private async initializeDiffAnalysis(graph: { nodes: any[]; edges: any[] }): Promise<void> {
        if (!this.workspaceRoot) return;

        try {
            // Create a mock parser for now - in production this would use the actual tree-sitter parser
            const mockParser = {
                parse: async (_code: string, _lang: string) => null as any
            };

            // Create diff analyzer if not exists
            if (!this.diffAnalyzer) {
                this.diffAnalyzer = createDiffAnalyzer(this.workspaceRoot, mockParser);

                // Set up diff change listener
                this.diffAnalyzer.onDiffChanged.on(({ nodeIds, graphDiff }) => {
                    this.sendDiffUpdate(graphDiff);
                });

                // Initialize with default baseline (HEAD~1)
                await this.diffAnalyzer.initializeDefaultBaseline();
            }

            // Create file watcher if not exists
            if (!this.diffFileWatcher && this.diffAnalyzer) {
                this.diffFileWatcher = createDiffFileWatcher(
                    this.diffAnalyzer,
                    this.workspaceRoot,
                    graph as any
                );
            } else if (this.diffFileWatcher) {
                // Update the graph reference
                this.diffFileWatcher.setGraph(graph as any);
            }

            // Perform initial diff analysis
            if (this.diffAnalyzer) {
                const graphDiff = await this.diffAnalyzer.analyzeGraph(graph as any);
                this.sendDiffUpdate(graphDiff);
            }
        } catch (error) {
            console.warn('Failed to initialize diff analysis:', error);
        }
    }

    /**
     * Send diff update to webview
     */
    private sendDiffUpdate(graphDiff: GraphDiff): void {
        const nodeDiffs: Record<string, NodeDiffView> = {};
        const edgeDiffs: Record<string, EdgeDiffView> = {};

        // Convert NodeDiff to NodeDiffView
        for (const [nodeId, diff] of graphDiff.nodeDiffs) {
            nodeDiffs[nodeId] = this.toNodeDiffView(diff);
        }

        // Convert EdgeDiff to EdgeDiffView
        for (const [edgeId, diff] of graphDiff.edgeDiffs) {
            edgeDiffs[edgeId] = this.toEdgeDiffView(diff);
        }

        this.postMessage({
            type: 'diff:update',
            payload: {
                nodeDiffs,
                edgeDiffs,
                addedNodes: graphDiff.addedNodes,
                removedNodes: graphDiff.removedNodes,
                baseline: {
                    type: graphDiff.baseline.type,
                    reference: graphDiff.baseline.reference,
                    label: graphDiff.baseline.label,
                },
            },
        });
    }

    /**
     * Convert NodeDiff to NodeDiffView for webview
     */
    private toNodeDiffView(diff: NodeDiff): NodeDiffView {
        return {
            nodeId: diff.nodeId,
            status: diff.status,
            severity: diff.overallSeverity,
            summary: {
                membersAdded: diff.summary.membersAdded,
                membersRemoved: diff.summary.membersRemoved,
                membersModified: diff.summary.membersModified,
                linesAdded: diff.summary.linesAdded,
                linesRemoved: diff.summary.linesRemoved,
            },
            memberDiffs: diff.memberDiffs.map(m => ({
                name: m.memberName,
                type: m.memberType as any,
                changeType: m.changeType,
                severity: m.severity,
                signatureChange: m.changes?.signatureChanged ? {
                    before: m.changes.beforeSignature || '',
                    after: m.changes.afterSignature || '',
                } : undefined,
                linesAdded: m.changes?.linesAdded || 0,
                linesRemoved: m.changes?.linesRemoved || 0,
            })),
        };
    }

    /**
     * Convert EdgeDiff to EdgeDiffView for webview
     */
    private toEdgeDiffView(diff: EdgeDiff): EdgeDiffView {
        return {
            edgeId: diff.edgeId,
            status: diff.status,
        };
    }

    /**
     * Send selection to agent
     */
    public async sendSelectionToAgent(
        selection: MapSelection,
        prompt?: string,
        outputFormat: OutputFormat = 'yaml'
    ): Promise<void> {
        if (!this.currentGraph) {
            vscode.window.showWarningMessage('No graph loaded');
            return;
        }

        if (selection.nodeIds.length === 0 && selection.edgeIds.length === 0) {
            vscode.window.showWarningMessage('No nodes or edges selected');
            return;
        }

        try {
            // Build agent payload using SelectionSerializer
            const agentPayload = await this.agentContextBuilder.buildContext(
                selection,
                prompt || '',
                outputFormat
            );

            // Get selection context for counting
            const selectionContext = await this.agentContextBuilder.getSelectionContext(selection);

            // Use existing kyco.sendToAgent infrastructure
            // This assumes the existing extension has this command or we integrate with HTTP endpoint
            const http = await import('http');

            const payload = JSON.stringify({
                type: 'code_map_selection',
                ...agentPayload,
            });

            const options = {
                hostname: 'localhost',
                port: 9876,
                path: '/agent',
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Content-Length': Buffer.byteLength(payload),
                },
            };

            const req = http.request(options, (res) => {
                if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
                    const nodeCount = selectionContext.nodes.length;
                    const edgeCount = selectionContext.edges.length;
                    const parts: string[] = [];
                    if (nodeCount > 0) parts.push(`${nodeCount} node(s)`);
                    if (edgeCount > 0) parts.push(`${edgeCount} edge(s)`);
                    vscode.window.showInformationMessage(
                        `Sent ${parts.join(' and ')} to agent`
                    );
                } else {
                    vscode.window.showErrorMessage(
                        `Agent server responded with status ${res.statusCode}`
                    );
                }
            });

            req.on('error', (error) => {
                vscode.window.showErrorMessage(
                    `Failed to send to agent: ${error.message}`
                );
            });

            req.write(payload);
            req.end();

        } catch (error) {
            vscode.window.showErrorMessage(`Failed to send to agent: ${error}`);
        }
    }

    /**
     * Set up message handler for webview communication
     */
    private setupMessageHandler(webview: vscode.Webview): void {
        webview.onDidReceiveMessage(
            async (message: WebviewToExtensionMessage) => {
                switch (message.type) {
                    case 'ready':
                        // Webview is ready, send initial graph
                        await this.refresh();
                        break;

                    case 'node:open':
                        await this.openNodeInEditor(message.payload);
                        break;

                    case 'selection:send':
                        await this.sendSelectionToAgent(
                            {
                                nodeIds: message.payload.nodeIds,
                                edgeIds: message.payload.edgeIds,
                                locRanges: message.payload.locRanges,
                                viewport: message.payload.viewport,
                            },
                            message.payload.prompt
                        );
                        break;

                    case 'graph:refresh':
                        await this.refresh();
                        break;
                }
            },
            null,
            this.disposables
        );
    }

    /**
     * Open a node's file in the editor
     */
    private async openNodeInEditor(payload: {
        nodeId: string;
        filePath: string;
        line: number;
    }): Promise<void> {
        try {
            const document = await vscode.workspace.openTextDocument(payload.filePath);
            const editor = await vscode.window.showTextDocument(document, {
                viewColumn: vscode.ViewColumn.One,
                preserveFocus: false,
            });

            // Go to line
            const position = new vscode.Position(Math.max(0, payload.line - 1), 0);
            editor.selection = new vscode.Selection(position, position);
            editor.revealRange(
                new vscode.Range(position, position),
                vscode.TextEditorRevealType.InCenter
            );

        } catch (error) {
            vscode.window.showErrorMessage(`Failed to open file: ${error}`);
        }
    }

    /**
     * Post message to webview
     */
    private postMessage(message: ExtensionToWebviewMessage): void {
        if (this._panel) {
            this._panel.webview.postMessage(message);
        } else if (this._view) {
            this._view.webview.postMessage(message);
        }
    }

    /**
     * Generate HTML for webview
     */
    private getHtmlForWebview(webview: vscode.Webview): string {
        // Get URIs for resources
        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'out', 'webview', 'webview.js')
        );
        const stylesUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'src', 'webview', 'styles.css')
        );
        const diffStylesUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'src', 'webview', 'diff', 'styles.css')
        );

        // Read HTML template and replace placeholders
        const htmlPath = vscode.Uri.joinPath(
            this.extensionUri,
            'src',
            'webview',
            'webview.html'
        );

        let html: string;
        try {
            html = fs.readFileSync(htmlPath.fsPath, 'utf-8');
        } catch {
            // Fallback HTML if template not found
            html = this.getFallbackHtml();
        }

        // Replace placeholders
        html = html
            .replace(/\${webview\.cspSource}/g, webview.cspSource)
            .replace(/\${scriptUri}/g, scriptUri.toString())
            .replace(/\${stylesUri}/g, stylesUri.toString())
            .replace(/\${diffStylesUri}/g, diffStylesUri.toString());

        return html;
    }

    /**
     * Fallback HTML if template not found
     */
    private getFallbackHtml(): string {
        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src \${webview.cspSource} 'unsafe-inline'; script-src \${webview.cspSource};">
    <title>Code Map</title>
    <link rel="stylesheet" href="\${stylesUri}">
</head>
<body>
    <div id="app">
        <div id="toolbar">
            <div class="toolbar-section">
                <span class="toolbar-label">Metrics:</span>
                <div class="button-group">
                    <button class="layer-btn active" data-layer="none">None</button>
                    <button class="layer-btn" data-layer="coverage">Coverage</button>
                    <button class="layer-btn" data-layer="loc">LOC</button>
                    <button class="layer-btn" data-layer="complexity">Complexity</button>
                </div>
            </div>
            <div class="toolbar-section">
                <button id="refresh-btn" class="action-btn">Refresh</button>
                <button id="send-btn" class="action-btn primary" disabled>Send to Agent</button>
            </div>
            <div class="toolbar-section selection-info">
                <span id="selection-count">0 selected</span>
            </div>
        </div>
        <div id="graph-container"></div>
        <div id="status-bar">
            <span id="status-nodes">Nodes: 0</span>
            <span id="status-edges">Edges: 0</span>
            <span id="status-zoom">Zoom: 100%</span>
        </div>
        <div id="legend"></div>
        <div id="coverage-legend" class="hidden"></div>
    </div>
    <script src="\${scriptUri}"></script>
</body>
</html>`;
    }

    /**
     * Dispose resources
     */
    public dispose(): void {
        this._panel?.dispose();
        this.disposables.forEach(d => d.dispose());
        this.diffFileWatcher?.dispose();
    }
}
