import * as fs from 'fs';
import * as path from 'path';
import Parser, { Language, SyntaxNode } from 'tree-sitter';
import { CoverageReport, FileCoverage, FunctionCoverage, GraphNode, NodeMetrics } from './types';
import { getAdapterForFile } from './adapters';

interface LcovRecord {
    file: string;
    lines: { line: number; hit: number }[];
    functions: { name: string; line: number; hit: number }[];
}

/**
 * MetricsCollector gathers and applies code metrics to graph nodes.
 * Supports coverage (lcov, cobertura, istanbul), LOC, and cyclomatic complexity.
 */
export class MetricsCollector {
    private coverageData: CoverageReport | null = null;
    private parsers: Map<string, Parser> = new Map();

    /**
     * Load coverage data from a report file
     */
    async loadCoverage(coveragePath: string): Promise<CoverageReport | null> {
        if (!coveragePath || !fs.existsSync(coveragePath)) {
            return null;
        }

        const ext = path.extname(coveragePath).toLowerCase();
        const content = await fs.promises.readFile(coveragePath, 'utf-8');

        try {
            if (coveragePath.endsWith('.info') || ext === '.lcov') {
                this.coverageData = this.parseLcov(content);
            } else if (ext === '.json') {
                // Istanbul JSON format
                this.coverageData = this.parseIstanbul(content);
            } else if (ext === '.xml') {
                // Cobertura XML format
                this.coverageData = this.parseCobertura(content);
            }
        } catch (error) {
            console.error('Error parsing coverage file:', error);
        }

        return this.coverageData;
    }

    /**
     * Calculate cyclomatic complexity for a file or code snippet
     */
    async calculateComplexity(filePath: string, sourceCode: string): Promise<Map<string, number>> {
        const complexityMap = new Map<string, number>();
        const adapter = getAdapterForFile(filePath);

        if (!adapter) {
            return complexityMap;
        }

        // Get or create parser
        let parser = this.parsers.get(adapter.languageId);
        if (!parser) {
            parser = new Parser();
            try {
                let grammar: Language;
                switch (adapter.languageId) {
                    case 'typescript':
                        grammar = require('tree-sitter-typescript').typescript;
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
                        return complexityMap;
                }
                parser.setLanguage(grammar);
                this.parsers.set(adapter.languageId, parser);
            } catch {
                return complexityMap;
            }
        }

        const tree = parser.parse(sourceCode);

        // Find all functions/methods
        const functionTypes = this.getFunctionNodeTypes(adapter.languageId);
        const functions = this.findNodesOfType(tree.rootNode, functionTypes);

        for (const func of functions) {
            const name = this.getFunctionName(func, adapter.languageId, sourceCode);
            if (!name) continue;

            const complexity = this.calculateNodeComplexity(func, adapter.languageId);
            complexityMap.set(name, complexity);
        }

        return complexityMap;
    }

    /**
     * Apply metrics to graph nodes
     */
    applyMetrics(
        nodes: GraphNode[],
        workspaceRoot: string,
        complexityMaps: Map<string, Map<string, number>>
    ): void {
        for (const node of nodes) {
            // Apply coverage
            if (this.coverageData && node.filePath) {
                const relativePath = path.relative(workspaceRoot, node.filePath);
                const fileCoverage = this.coverageData.files.get(relativePath) ||
                    this.coverageData.files.get(node.filePath);

                if (fileCoverage) {
                    if (node.type === 'file') {
                        node.metrics.coverage = fileCoverage.percentage;
                    } else if (node.type === 'function' || node.type === 'method') {
                        // Try to find function-level coverage
                        const funcCoverage = fileCoverage.functions.find(
                            f => f.name === node.label ||
                                (f.lineStart >= node.lineStart && f.lineEnd <= node.lineEnd)
                        );
                        if (funcCoverage) {
                            node.metrics.coverage = funcCoverage.percentage;
                        } else {
                            // Estimate from line coverage
                            node.metrics.coverage = fileCoverage.percentage;
                        }
                    }
                }
            }

            // Apply complexity
            if (complexityMaps.has(node.filePath)) {
                const fileComplexity = complexityMaps.get(node.filePath)!;
                if (node.type === 'function' || node.type === 'method') {
                    const complexity = fileComplexity.get(node.label);
                    if (complexity !== undefined) {
                        node.metrics.complexity = complexity;
                    }
                } else if (node.type === 'file') {
                    // Sum all function complexities for file-level
                    let totalComplexity = 0;
                    for (const c of fileComplexity.values()) {
                        totalComplexity += c;
                    }
                    node.metrics.complexity = totalComplexity;
                }
            }
        }
    }

    private parseLcov(content: string): CoverageReport {
        const files = new Map<string, FileCoverage>();
        let totalCovered = 0;
        let totalLines = 0;

        const records = this.parseLcovRecords(content);

        for (const record of records) {
            const linesCovered = record.lines.filter(l => l.hit > 0).length;
            const linesTotal = record.lines.length;

            const functions: FunctionCoverage[] = record.functions.map(f => ({
                name: f.name,
                lineStart: f.line,
                lineEnd: f.line,
                hitCount: f.hit,
                percentage: f.hit > 0 ? 100 : 0,
            }));

            files.set(record.file, {
                filePath: record.file,
                linesCovered,
                linesTotal,
                percentage: linesTotal > 0 ? (linesCovered / linesTotal) * 100 : 0,
                functions,
            });

            totalCovered += linesCovered;
            totalLines += linesTotal;
        }

        return {
            files,
            totalCoverage: totalLines > 0 ? (totalCovered / totalLines) * 100 : 0,
        };
    }

    private parseLcovRecords(content: string): LcovRecord[] {
        const records: LcovRecord[] = [];
        let current: LcovRecord | null = null;

        const lines = content.split('\n');

        for (const line of lines) {
            const trimmed = line.trim();

            if (trimmed.startsWith('SF:')) {
                current = {
                    file: trimmed.slice(3),
                    lines: [],
                    functions: [],
                };
            } else if (trimmed.startsWith('DA:') && current) {
                const [lineNum, hit] = trimmed.slice(3).split(',').map(Number);
                current.lines.push({ line: lineNum, hit });
            } else if (trimmed.startsWith('FN:') && current) {
                const [lineNum, name] = trimmed.slice(3).split(',');
                current.functions.push({ name, line: parseInt(lineNum, 10), hit: 0 });
            } else if (trimmed.startsWith('FNDA:') && current) {
                const [hit, name] = trimmed.slice(5).split(',');
                const func = current.functions.find(f => f.name === name);
                if (func) {
                    func.hit = parseInt(hit, 10);
                }
            } else if (trimmed === 'end_of_record' && current) {
                records.push(current);
                current = null;
            }
        }

        return records;
    }

    private parseIstanbul(content: string): CoverageReport {
        const files = new Map<string, FileCoverage>();
        let totalCovered = 0;
        let totalLines = 0;

        try {
            const data = JSON.parse(content);

            for (const [filePath, fileCov] of Object.entries(data)) {
                const cov = fileCov as any;

                // Count statement coverage
                const statements = Object.values(cov.s || {}) as number[];
                const linesCovered = statements.filter(s => s > 0).length;
                const linesTotal = statements.length;

                // Extract function coverage
                const functions: FunctionCoverage[] = [];
                const fnMap = cov.fnMap || {};
                const fnCounts = cov.f || {};

                for (const [fnId, fnData] of Object.entries(fnMap)) {
                    const fn = fnData as any;
                    const hitCount = fnCounts[fnId] || 0;

                    functions.push({
                        name: fn.name,
                        lineStart: fn.loc?.start?.line || fn.line || 0,
                        lineEnd: fn.loc?.end?.line || fn.line || 0,
                        hitCount,
                        percentage: hitCount > 0 ? 100 : 0,
                    });
                }

                files.set(filePath, {
                    filePath,
                    linesCovered,
                    linesTotal,
                    percentage: linesTotal > 0 ? (linesCovered / linesTotal) * 100 : 0,
                    functions,
                });

                totalCovered += linesCovered;
                totalLines += linesTotal;
            }
        } catch (error) {
            console.error('Error parsing Istanbul coverage:', error);
        }

        return {
            files,
            totalCoverage: totalLines > 0 ? (totalCovered / totalLines) * 100 : 0,
        };
    }

    private parseCobertura(content: string): CoverageReport {
        const files = new Map<string, FileCoverage>();
        let totalCovered = 0;
        let totalLines = 0;

        // Simple XML parsing for Cobertura format
        // In production, use a proper XML parser
        const classRegex = /<class[^>]*filename="([^"]*)"[^>]*line-rate="([^"]*)"[^>]*>/g;
        const methodRegex = /<method[^>]*name="([^"]*)"[^>]*line-rate="([^"]*)"[^>]*>/g;

        let match;
        while ((match = classRegex.exec(content)) !== null) {
            const filePath = match[1];
            const lineRate = parseFloat(match[2]);
            const percentage = lineRate * 100;

            // Extract methods for this class
            const functions: FunctionCoverage[] = [];
            let methodMatch;
            while ((methodMatch = methodRegex.exec(content)) !== null) {
                functions.push({
                    name: methodMatch[1],
                    lineStart: 0,
                    lineEnd: 0,
                    hitCount: parseFloat(methodMatch[2]) > 0 ? 1 : 0,
                    percentage: parseFloat(methodMatch[2]) * 100,
                });
            }

            files.set(filePath, {
                filePath,
                linesCovered: 0,
                linesTotal: 0,
                percentage,
                functions,
            });

            totalCovered += percentage;
            totalLines += 100;
        }

        return {
            files,
            totalCoverage: totalLines > 0 ? totalCovered / (totalLines / 100) : 0,
        };
    }

    private getFunctionNodeTypes(languageId: string): string[] {
        switch (languageId) {
            case 'typescript':
                return ['function_declaration', 'arrow_function', 'method_definition'];
            case 'python':
                return ['function_definition'];
            case 'csharp':
                return ['method_declaration', 'constructor_declaration'];
            case 'rust':
                return ['function_item'];
            case 'go':
                return ['function_declaration', 'method_declaration'];
            default:
                return [];
        }
    }

    private findNodesOfType(node: SyntaxNode, types: string[]): SyntaxNode[] {
        const results: SyntaxNode[] = [];

        const traverse = (n: SyntaxNode) => {
            if (types.includes(n.type)) {
                results.push(n);
            }
            for (let i = 0; i < n.childCount; i++) {
                const child = n.child(i);
                if (child) traverse(child);
            }
        };

        traverse(node);
        return results;
    }

    private getFunctionName(
        node: SyntaxNode,
        languageId: string,
        sourceCode: string
    ): string | null {
        const nameNode = node.childForFieldName('name');
        if (nameNode) {
            return sourceCode.slice(nameNode.startIndex, nameNode.endIndex);
        }

        // For arrow functions, check parent variable declarator
        if (node.parent?.type === 'variable_declarator') {
            const varName = node.parent.childForFieldName('name');
            if (varName) {
                return sourceCode.slice(varName.startIndex, varName.endIndex);
            }
        }

        return null;
    }

    private calculateNodeComplexity(node: SyntaxNode, languageId: string): number {
        // Base complexity is 1
        let complexity = 1;

        // Decision points that increase complexity
        const decisionTypes = this.getDecisionNodeTypes(languageId);

        const traverse = (n: SyntaxNode) => {
            if (decisionTypes.includes(n.type)) {
                complexity++;
            }
            for (let i = 0; i < n.childCount; i++) {
                const child = n.child(i);
                if (child) traverse(child);
            }
        };

        traverse(node);
        return complexity;
    }

    private getDecisionNodeTypes(languageId: string): string[] {
        // Common decision points across languages
        const common = [
            'if_statement', 'if_expression',
            'while_statement', 'while_expression',
            'for_statement', 'for_expression',
            'switch_statement', 'switch_expression',
            'case_clause', 'case',
            'catch_clause', 'except_clause',
            'ternary_expression', 'conditional_expression',
            '&&', '||', 'and', 'or',
        ];

        switch (languageId) {
            case 'typescript':
                return [...common, 'optional_chain'];
            case 'python':
                return [...common, 'elif_clause', 'list_comprehension', 'generator_expression'];
            case 'csharp':
                return [...common, 'switch_expression_arm', 'when_clause'];
            case 'rust':
                return [...common, 'match_expression', 'match_arm', 'if_let_expression'];
            case 'go':
                return [...common, 'select_statement', 'comm_clause'];
            default:
                return common;
        }
    }
}
