import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import Parser, { Language } from 'tree-sitter';
import {
    GraphNode,
    GraphEdge,
    CodeGraph,
    GraphMetadata,
    NodeType,
    VisNode,
    VisEdge,
    VisGraph,
} from './types';
import { getAdapterForFile, getSupportedExtensions } from './adapters';
import { LanguageAdapter, ExtractedSymbol, ExtractedImport } from './LanguageAdapter';

interface BuildOptions {
    workspaceRoot: string;
    includePatterns: string[];
    excludePatterns: string[];
}

interface ParsedFile {
    filePath: string;
    relativePath: string;
    language: string;
    symbols: ExtractedSymbol[];
    imports: ExtractedImport[];
    sourceCode: string;
}

/**
 * GraphBuilder orchestrates the parsing of a codebase and construction
 * of the dependency graph using Tree-sitter and language-specific adapters.
 */
export class GraphBuilder {
    private parsers: Map<string, Parser> = new Map();
    private nodeIdCounter = 0;

    constructor() {
        // Parsers are initialized lazily
    }

    /**
     * Build the complete code graph for a workspace
     */
    async buildGraph(options: BuildOptions): Promise<CodeGraph> {
        const { workspaceRoot, includePatterns, excludePatterns } = options;

        // Find all matching files
        const files = await this.findFiles(workspaceRoot, includePatterns, excludePatterns);

        // Parse all files
        const parsedFiles = await this.parseFiles(files, workspaceRoot);

        // Build nodes and edges
        const { nodes, edges } = this.buildNodesAndEdges(parsedFiles, workspaceRoot);

        // Create metadata
        const languages = [...new Set(parsedFiles.map(f => f.language))];
        const metadata: GraphMetadata = {
            generatedAt: Date.now(),
            workspaceRoot,
            languages,
            nodeCount: nodes.length,
            edgeCount: edges.length,
        };

        return { nodes, edges, metadata };
    }

    /**
     * Convert CodeGraph to vis-network compatible format
     */
    toVisGraph(graph: CodeGraph): VisGraph {
        const visNodes: VisNode[] = graph.nodes.map(node => this.toVisNode(node));
        const visEdges: VisEdge[] = graph.edges.map(edge => this.toVisEdge(edge));
        return { nodes: visNodes, edges: visEdges };
    }

    private async findFiles(
        workspaceRoot: string,
        includePatterns: string[],
        excludePatterns: string[]
    ): Promise<string[]> {
        const files: string[] = [];

        // Use VS Code's file search API
        for (const pattern of includePatterns) {
            const excludePattern = excludePatterns.join(',');
            const uris = await vscode.workspace.findFiles(
                new vscode.RelativePattern(workspaceRoot, pattern),
                `{${excludePattern}}`
            );
            files.push(...uris.map(uri => uri.fsPath));
        }

        // Deduplicate
        return [...new Set(files)];
    }

    private async parseFiles(files: string[], workspaceRoot: string): Promise<ParsedFile[]> {
        const results: ParsedFile[] = [];

        for (const filePath of files) {
            const adapter = getAdapterForFile(filePath);
            if (!adapter) continue;

            try {
                const sourceCode = await fs.promises.readFile(filePath, 'utf-8');
                const parser = await this.getParser(adapter);
                const tree = parser.parse(sourceCode);

                const symbols = adapter.extractSymbols(tree, sourceCode);
                const imports = adapter.extractImports(tree, sourceCode);

                results.push({
                    filePath,
                    relativePath: path.relative(workspaceRoot, filePath),
                    language: adapter.languageId,
                    symbols,
                    imports,
                    sourceCode,
                });
            } catch (error) {
                console.error(`Error parsing ${filePath}:`, error);
            }
        }

        return results;
    }

    private async getParser(adapter: LanguageAdapter): Promise<Parser> {
        const existing = this.parsers.get(adapter.languageId);
        if (existing) return existing;

        const parser = new Parser();

        // Load the appropriate grammar
        try {
            let grammar: Language;

            switch (adapter.languageId) {
                case 'typescript':
                    // TypeScript grammar includes both TS and TSX
                    const tsGrammar = require('tree-sitter-typescript');
                    grammar = tsGrammar.typescript;
                    break;
                case 'python':
                    grammar = require('tree-sitter-python');
                    break;
                case 'csharp':
                    grammar = require('tree-sitter-c-sharp');
                    break;
                case 'rust':
                    grammar = require('tree-sitter-rust');
                    break;
                case 'go':
                    grammar = require('tree-sitter-go');
                    break;
                default:
                    throw new Error(`No grammar for language: ${adapter.languageId}`);
            }

            parser.setLanguage(grammar);
            this.parsers.set(adapter.languageId, parser);
            return parser;
        } catch (error) {
            console.error(`Error loading grammar for ${adapter.languageId}:`, error);
            throw error;
        }
    }

    private buildNodesAndEdges(
        parsedFiles: ParsedFile[],
        workspaceRoot: string
    ): { nodes: GraphNode[]; edges: GraphEdge[] } {
        const nodes: GraphNode[] = [];
        const edges: GraphEdge[] = [];

        // Map to track node IDs by file path and symbol name
        const nodeIdMap = new Map<string, string>();
        const fileNodeMap = new Map<string, string>();

        // First pass: Create directory and file nodes
        const directories = new Set<string>();

        for (const file of parsedFiles) {
            // Collect all directory paths
            const parts = file.relativePath.split(path.sep);
            let currentPath = '';
            for (let i = 0; i < parts.length - 1; i++) {
                currentPath = currentPath ? path.join(currentPath, parts[i]) : parts[i];
                directories.add(currentPath);
            }

            // Create file node
            const fileNodeId = this.generateNodeId();
            const fileName = path.basename(file.filePath);
            const loc = file.sourceCode.split('\n').length;

            nodes.push({
                id: fileNodeId,
                label: fileName,
                type: 'file',
                language: file.language,
                filePath: file.filePath,
                lineStart: 1,
                lineEnd: loc,
                isExported: true,
                parentId: null, // Will be set later
                metrics: { loc },
            });

            fileNodeMap.set(file.relativePath, fileNodeId);
            nodeIdMap.set(file.filePath, fileNodeId);
        }

        // Create directory nodes
        const dirNodeMap = new Map<string, string>();
        const sortedDirs = [...directories].sort((a, b) => a.split(path.sep).length - b.split(path.sep).length);

        for (const dir of sortedDirs) {
            const dirNodeId = this.generateNodeId();
            const dirName = path.basename(dir);
            const parentDir = path.dirname(dir);

            nodes.push({
                id: dirNodeId,
                label: dirName,
                type: 'directory',
                language: '',
                filePath: path.join(workspaceRoot, dir),
                lineStart: 0,
                lineEnd: 0,
                isExported: true,
                parentId: parentDir !== '.' ? dirNodeMap.get(parentDir) || null : null,
                metrics: { loc: 0 },
            });

            dirNodeMap.set(dir, dirNodeId);
        }

        // Link file nodes to their parent directories
        for (const file of parsedFiles) {
            const fileNodeId = fileNodeMap.get(file.relativePath);
            if (!fileNodeId) continue;

            const parentDir = path.dirname(file.relativePath);
            const parentNodeId = parentDir !== '.' ? dirNodeMap.get(parentDir) : null;

            const fileNode = nodes.find(n => n.id === fileNodeId);
            if (fileNode && parentNodeId) {
                fileNode.parentId = parentNodeId;

                // Create contains edge
                edges.push({
                    id: this.generateEdgeId(),
                    from: parentNodeId,
                    to: fileNodeId,
                    type: 'contains',
                });
            }
        }

        // Second pass: Create symbol nodes and contains edges
        for (const file of parsedFiles) {
            const fileNodeId = fileNodeMap.get(file.relativePath);
            if (!fileNodeId) continue;

            // Track parent classes for methods
            const classNodeIds = new Map<string, string>();

            for (const symbol of file.symbols) {
                const symbolNodeId = this.generateNodeId();
                const loc = symbol.lineEnd - symbol.lineStart + 1;

                // Determine parent
                let parentId: string | null = fileNodeId;
                if (symbol.parentName && classNodeIds.has(symbol.parentName)) {
                    parentId = classNodeIds.get(symbol.parentName)!;
                }

                nodes.push({
                    id: symbolNodeId,
                    label: symbol.name,
                    type: symbol.type,
                    language: file.language,
                    filePath: file.filePath,
                    lineStart: symbol.lineStart,
                    lineEnd: symbol.lineEnd,
                    isExported: symbol.isExported,
                    parentId,
                    signature: symbol.signature,
                    metrics: { loc },
                });

                // Track class nodes for method parenting
                if (symbol.type === 'class' || symbol.type === 'interface') {
                    classNodeIds.set(symbol.name, symbolNodeId);
                }

                // Create contains edge from parent
                if (parentId) {
                    edges.push({
                        id: this.generateEdgeId(),
                        from: parentId,
                        to: symbolNodeId,
                        type: 'contains',
                    });
                }

                // Store in nodeIdMap for import resolution
                const key = `${file.filePath}:${symbol.name}`;
                nodeIdMap.set(key, symbolNodeId);
            }
        }

        // Third pass: Create import edges
        for (const file of parsedFiles) {
            const fileNodeId = fileNodeMap.get(file.relativePath);
            if (!fileNodeId) continue;

            for (const imp of file.imports) {
                // Resolve import path to actual file
                const resolvedPath = this.resolveImportPath(
                    imp.importPath,
                    file.filePath,
                    workspaceRoot,
                    parsedFiles
                );

                if (resolvedPath) {
                    const targetFileNodeId = nodeIdMap.get(resolvedPath);
                    if (targetFileNodeId && targetFileNodeId !== fileNodeId) {
                        edges.push({
                            id: this.generateEdgeId(),
                            from: fileNodeId,
                            to: targetFileNodeId,
                            type: 'import',
                            label: imp.importedNames.join(', '),
                        });
                    }
                }
            }
        }

        return { nodes, edges };
    }

    private resolveImportPath(
        importPath: string,
        fromFile: string,
        workspaceRoot: string,
        parsedFiles: ParsedFile[]
    ): string | null {
        // Skip external/node_modules imports
        if (!importPath.startsWith('.') && !importPath.startsWith('/')) {
            return null;
        }

        const fromDir = path.dirname(fromFile);
        const extensions = getSupportedExtensions();

        // Try to resolve with various extensions
        const candidates: string[] = [];

        if (importPath.startsWith('.')) {
            const basePath = path.resolve(fromDir, importPath);
            candidates.push(basePath);

            // Try with extensions
            for (const ext of extensions) {
                candidates.push(basePath + ext);
            }

            // Try as directory with index file
            for (const ext of extensions) {
                candidates.push(path.join(basePath, `index${ext}`));
            }
        }

        // Find matching parsed file
        for (const candidate of candidates) {
            const normalized = path.normalize(candidate);
            const found = parsedFiles.find(f => path.normalize(f.filePath) === normalized);
            if (found) {
                return found.filePath;
            }
        }

        return null;
    }

    private generateNodeId(): string {
        return `node_${++this.nodeIdCounter}`;
    }

    private generateEdgeId(): string {
        return `edge_${this.nodeIdCounter++}`;
    }

    private toVisNode(node: GraphNode): VisNode {
        const shapeMap: Record<NodeType, string> = {
            directory: 'box',
            file: 'box',
            namespace: 'box',
            class: 'box',
            interface: 'diamond',
            function: 'ellipse',
            method: 'ellipse',
            export: 'triangle',
        };

        const colorMap: Record<NodeType, { background: string; border: string }> = {
            directory: { background: '#e8e8e8', border: '#999999' },
            file: { background: '#d4e5f7', border: '#4a90d9' },
            namespace: { background: '#f0e6d3', border: '#c9a959' },
            class: { background: '#d4f7d4', border: '#4a9d4a' },
            interface: { background: '#f7e6d4', border: '#d99a4a' },
            function: { background: '#e6d4f7', border: '#9a4ad9' },
            method: { background: '#f7d4e6', border: '#d94a9a' },
            export: { background: '#d4f7f7', border: '#4a9d9d' },
        };

        // Determine level based on type for hierarchical layout
        const levelMap: Record<NodeType, number> = {
            directory: 0,
            namespace: 1,
            file: 1,
            class: 2,
            interface: 2,
            function: 3,
            method: 3,
            export: 3,
        };

        return {
            id: node.id,
            label: node.label,
            title: this.buildNodeTooltip(node),
            group: node.type,
            level: levelMap[node.type],
            shape: shapeMap[node.type],
            color: colorMap[node.type],
            size: Math.max(10, Math.min(50, Math.sqrt(node.metrics.loc) * 3)),
            borderWidth: node.isExported ? 2 : 1,
            font: { size: 12, color: '#333333' },
            data: node,
        };
    }

    private toVisEdge(edge: GraphEdge): VisEdge {
        const styleMap: Record<string, { arrows: string; dashes: boolean; color: string }> = {
            import: { arrows: 'to', dashes: false, color: '#4a90d9' },
            call: { arrows: 'to', dashes: true, color: '#9a4ad9' },
            contains: { arrows: '', dashes: false, color: '#cccccc' },
            inheritance: { arrows: 'to', dashes: false, color: '#4a9d4a' },
            implementation: { arrows: 'to', dashes: true, color: '#d99a4a' },
        };

        const style = styleMap[edge.type] || styleMap.import;

        return {
            id: edge.id,
            from: edge.from,
            to: edge.to,
            label: edge.type === 'import' ? edge.label : undefined,
            arrows: style.arrows,
            dashes: style.dashes,
            color: { color: style.color, opacity: 0.8 },
            width: edge.type === 'contains' ? 1 : 2,
            data: edge,
        };
    }

    private buildNodeTooltip(node: GraphNode): string {
        const lines: string[] = [
            `<b>${node.label}</b>`,
            `Type: ${node.type}`,
            `Language: ${node.language || 'N/A'}`,
            `Lines: ${node.lineStart}-${node.lineEnd}`,
            `LOC: ${node.metrics.loc}`,
        ];

        if (node.signature) {
            lines.push(`Signature: ${node.signature}`);
        }

        if (node.metrics.coverage !== undefined) {
            lines.push(`Coverage: ${node.metrics.coverage.toFixed(1)}%`);
        }

        if (node.metrics.complexity !== undefined) {
            lines.push(`Complexity: ${node.metrics.complexity}`);
        }

        return lines.join('<br>');
    }
}
