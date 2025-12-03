/**
 * ASTDiffEngine - Compares ASTs to extract structural changes
 * Works with tree-sitter parsed ASTs through language adapters
 */

import type { SyntaxNode, Tree } from 'tree-sitter';
import type { LanguageAdapter, ExtractedSymbol, ExtractedImport } from '../graph/LanguageAdapter';
import type {
    NodeDiff,
    MemberDiff,
    MemberInfo,
    MemberType,
    DependencyChanges,
    InheritanceChanges,
    ChangeStatus,
    DiffSeverity,
} from './types';
import { computeBodyHash } from './DiffCache';
import {
    classifyAddedMember,
    classifyRemovedMember,
    classifyModifiedMember,
    calculateLineDiff,
    summarizeNodeChanges,
    calculateNodeSeverity,
} from './severity';

/**
 * Interface for AST diff operations
 */
export interface IASTDiffEngine {
    /**
     * Compare two parsed ASTs and extract node-level diffs
     */
    diffNodes(
        beforeTree: Tree | null,
        afterTree: Tree | null,
        beforeSource: string,
        afterSource: string,
        adapter: LanguageAdapter
    ): NodeDiff[];

    /**
     * Compare member lists to get member-level diffs
     */
    diffMembers(
        beforeMembers: MemberInfo[],
        afterMembers: MemberInfo[]
    ): MemberDiff[];
}

/**
 * AST Diff Engine implementation
 */
export class ASTDiffEngine implements IASTDiffEngine {
    /**
     * Compare two ASTs and extract all node diffs
     */
    diffNodes(
        beforeTree: Tree | null,
        afterTree: Tree | null,
        beforeSource: string,
        afterSource: string,
        adapter: LanguageAdapter
    ): NodeDiff[] {
        const diffs: NodeDiff[] = [];

        // Extract symbols from both trees
        const beforeSymbols = beforeTree ? adapter.extractSymbols(beforeTree, beforeSource) : [];
        const afterSymbols = afterTree ? adapter.extractSymbols(afterTree, afterSource) : [];

        // Extract imports
        const beforeImports = beforeTree ? adapter.extractImports(beforeTree, beforeSource) : [];
        const afterImports = afterTree ? adapter.extractImports(afterTree, afterSource) : [];

        // Group by top-level symbols (classes, functions, interfaces)
        const beforeMap = this.groupSymbolsByName(beforeSymbols);
        const afterMap = this.groupSymbolsByName(afterSymbols);

        // Find added nodes (in after but not in before)
        for (const [name, afterSymbol] of afterMap) {
            if (!beforeMap.has(name)) {
                diffs.push(this.createAddedNodeDiff(
                    afterSymbol,
                    afterSource,
                    adapter
                ));
            }
        }

        // Find removed nodes (in before but not in after)
        for (const [name, beforeSymbol] of beforeMap) {
            if (!afterMap.has(name)) {
                diffs.push(this.createRemovedNodeDiff(
                    beforeSymbol,
                    beforeSource,
                    adapter
                ));
            }
        }

        // Find modified nodes (in both)
        for (const [name, afterSymbol] of afterMap) {
            const beforeSymbol = beforeMap.get(name);
            if (beforeSymbol) {
                const diff = this.compareSymbols(
                    beforeSymbol,
                    afterSymbol,
                    beforeSource,
                    afterSource,
                    beforeImports,
                    afterImports,
                    adapter
                );
                if (diff.status !== 'unchanged') {
                    diffs.push(diff);
                }
            }
        }

        return diffs;
    }

    /**
     * Compare member lists
     */
    diffMembers(
        beforeMembers: MemberInfo[],
        afterMembers: MemberInfo[]
    ): MemberDiff[] {
        const diffs: MemberDiff[] = [];
        const beforeMap = new Map(beforeMembers.map(m => [m.name, m]));
        const afterMap = new Map(afterMembers.map(m => [m.name, m]));

        // Removed members
        for (const [name, member] of beforeMap) {
            if (!afterMap.has(name)) {
                diffs.push({
                    memberName: name,
                    memberType: member.type,
                    changeType: 'removed',
                    severity: classifyRemovedMember(member),
                });
            }
        }

        // Added members
        for (const [name, member] of afterMap) {
            if (!beforeMap.has(name)) {
                diffs.push({
                    memberName: name,
                    memberType: member.type,
                    changeType: 'added',
                    severity: classifyAddedMember(member),
                });
            }
        }

        // Modified members
        for (const [name, afterMember] of afterMap) {
            const beforeMember = beforeMap.get(name);
            if (beforeMember) {
                const memberDiff = this.compareMember(beforeMember, afterMember);
                if (memberDiff) {
                    diffs.push(memberDiff);
                }
            }
        }

        return diffs;
    }

    /**
     * Create diff for a newly added node
     */
    private createAddedNodeDiff(
        symbol: ExtractedSymbol,
        sourceCode: string,
        adapter: LanguageAdapter
    ): NodeDiff {
        const members = this.extractMembersForSymbol(symbol, sourceCode, adapter);

        return {
            nodeId: this.generateNodeId(symbol),
            filePath: '', // Will be set by caller
            nodeType: symbol.type as NodeDiff['nodeType'],
            nodeName: symbol.name,
            status: 'added',
            overallSeverity: symbol.isExported ? 'high' : 'medium',
            summary: {
                membersAdded: members.length,
                membersRemoved: 0,
                membersModified: 0,
                linesAdded: symbol.lineEnd - symbol.lineStart + 1,
                linesRemoved: 0,
                signatureChanges: 0,
            },
            memberDiffs: members.map(m => ({
                memberName: m.name,
                memberType: m.type,
                changeType: 'added' as ChangeStatus,
                severity: classifyAddedMember(m),
            })),
            dependencyChanges: { added: [], removed: [] },
        };
    }

    /**
     * Create diff for a removed node
     */
    private createRemovedNodeDiff(
        symbol: ExtractedSymbol,
        sourceCode: string,
        adapter: LanguageAdapter
    ): NodeDiff {
        const members = this.extractMembersForSymbol(symbol, sourceCode, adapter);

        return {
            nodeId: this.generateNodeId(symbol),
            filePath: '', // Will be set by caller
            nodeType: symbol.type as NodeDiff['nodeType'],
            nodeName: symbol.name,
            status: 'removed',
            overallSeverity: symbol.isExported ? 'high' : 'medium',
            summary: {
                membersAdded: 0,
                membersRemoved: members.length,
                membersModified: 0,
                linesAdded: 0,
                linesRemoved: symbol.lineEnd - symbol.lineStart + 1,
                signatureChanges: 0,
            },
            memberDiffs: members.map(m => ({
                memberName: m.name,
                memberType: m.type,
                changeType: 'removed' as ChangeStatus,
                severity: classifyRemovedMember(m),
            })),
            dependencyChanges: { added: [], removed: [] },
        };
    }

    /**
     * Compare two versions of the same symbol
     */
    private compareSymbols(
        beforeSymbol: ExtractedSymbol,
        afterSymbol: ExtractedSymbol,
        beforeSource: string,
        afterSource: string,
        beforeImports: ExtractedImport[],
        afterImports: ExtractedImport[],
        adapter: LanguageAdapter
    ): NodeDiff {
        // Extract members from both versions
        const beforeMembers = this.extractMembersForSymbol(beforeSymbol, beforeSource, adapter);
        const afterMembers = this.extractMembersForSymbol(afterSymbol, afterSource, adapter);

        // Diff members
        const memberDiffs = this.diffMembers(beforeMembers, afterMembers);

        // Diff dependencies
        const dependencyChanges = this.diffDependencies(beforeImports, afterImports);

        // Diff inheritance (for classes)
        const inheritanceChanges = this.diffInheritance(beforeSymbol, afterSymbol);

        // Determine overall severity
        const overallSeverity = calculateNodeSeverity(
            memberDiffs,
            dependencyChanges,
            inheritanceChanges
        );

        // Determine status
        let status: ChangeStatus = 'unchanged';
        if (memberDiffs.length > 0 ||
            dependencyChanges.added.length > 0 ||
            dependencyChanges.removed.length > 0 ||
            inheritanceChanges) {
            status = 'modified';
        }

        return {
            nodeId: this.generateNodeId(afterSymbol),
            filePath: '', // Will be set by caller
            nodeType: afterSymbol.type as NodeDiff['nodeType'],
            nodeName: afterSymbol.name,
            status,
            overallSeverity,
            summary: summarizeNodeChanges(memberDiffs),
            memberDiffs,
            dependencyChanges,
            inheritanceChanges,
        };
    }

    /**
     * Compare two members
     */
    private compareMember(before: MemberInfo, after: MemberInfo): MemberDiff | null {
        const signatureChanged = before.signature !== after.signature;
        const bodyChanged = before.bodyHash !== after.bodyHash;

        if (!signatureChanged && !bodyChanged) {
            return null; // No change
        }

        const severity = classifyModifiedMember(before, after);
        const lineDiff = calculateLineDiff(before.rawText, after.rawText);

        return {
            memberName: before.name,
            memberType: before.type,
            changeType: 'modified',
            severity,
            changes: {
                signatureChanged,
                beforeSignature: signatureChanged ? before.signature : undefined,
                afterSignature: signatureChanged ? after.signature : undefined,
                bodyChanged,
                linesAdded: lineDiff.added,
                linesRemoved: lineDiff.removed,
            },
        };
    }

    /**
     * Extract members for a symbol
     */
    private extractMembersForSymbol(
        symbol: ExtractedSymbol,
        sourceCode: string,
        adapter: LanguageAdapter
    ): MemberInfo[] {
        // Only extract members for classes and interfaces
        if (symbol.type !== 'class' && symbol.type !== 'interface') {
            // For functions, treat the function itself as a single "member"
            if (symbol.type === 'function') {
                const rawText = this.extractSourceLines(
                    sourceCode,
                    symbol.lineStart,
                    symbol.lineEnd
                );
                return [{
                    name: symbol.name,
                    type: 'method',
                    signature: symbol.signature || `function ${symbol.name}()`,
                    bodyHash: computeBodyHash(rawText),
                    rawText,
                    lineStart: symbol.lineStart,
                    lineEnd: symbol.lineEnd,
                    isExported: symbol.isExported,
                }];
            }
            return [];
        }

        // For classes/interfaces, extract method members
        // This is a simplified version - full implementation would use AST
        return this.extractMembersFromSource(
            sourceCode,
            symbol.lineStart,
            symbol.lineEnd,
            adapter.languageId
        );
    }

    /**
     * Extract members from source code lines
     * Simplified version using regex patterns
     */
    private extractMembersFromSource(
        sourceCode: string,
        startLine: number,
        endLine: number,
        language: string
    ): MemberInfo[] {
        const members: MemberInfo[] = [];
        const lines = sourceCode.split('\n');
        const classContent = lines.slice(startLine - 1, endLine).join('\n');

        // Language-specific patterns
        const patterns = this.getMemberPatterns(language);

        for (const pattern of patterns) {
            const regex = new RegExp(pattern.regex, 'gm');
            let match;

            while ((match = regex.exec(classContent)) !== null) {
                const name = match[pattern.nameGroup];
                const fullMatch = match[0];

                // Find the body
                const bodyStart = classContent.indexOf(fullMatch);
                const body = this.extractMemberBody(classContent, bodyStart);

                members.push({
                    name,
                    type: pattern.type,
                    signature: this.extractSignature(fullMatch, pattern.type),
                    bodyHash: computeBodyHash(body),
                    rawText: fullMatch + body,
                    lineStart: startLine + this.countLinesBefore(classContent, bodyStart),
                    lineEnd: startLine + this.countLinesBefore(classContent, bodyStart + fullMatch.length + body.length),
                    isExported: this.isPublicMember(fullMatch, language),
                });
            }
        }

        return members;
    }

    /**
     * Get regex patterns for member extraction based on language
     */
    private getMemberPatterns(language: string): { regex: string; nameGroup: number; type: MemberType }[] {
        switch (language) {
            case 'typescript':
            case 'javascript':
                return [
                    // Methods: name(...) { or async name(...) {
                    { regex: '(?:async\\s+)?([a-zA-Z_$][a-zA-Z0-9_$]*)\\s*\\([^)]*\\)\\s*(?::\\s*[^{]+)?\\s*\\{', nameGroup: 1, type: 'method' },
                    // Getters: get name() {
                    { regex: 'get\\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\\s*\\(\\s*\\)\\s*(?::\\s*[^{]+)?\\s*\\{', nameGroup: 1, type: 'getter' },
                    // Setters: set name(value) {
                    { regex: 'set\\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\\s*\\([^)]*\\)\\s*\\{', nameGroup: 1, type: 'setter' },
                    // Properties: name: type = or name =
                    { regex: '(?:public|private|protected)?\\s*([a-zA-Z_$][a-zA-Z0-9_$]*)\\s*(?::\\s*[^=;]+)?\\s*=', nameGroup: 1, type: 'property' },
                    // Constructor
                    { regex: 'constructor\\s*\\([^)]*\\)\\s*\\{', nameGroup: 0, type: 'constructor' },
                ];
            case 'python':
                return [
                    // Methods: def name(self, ...):
                    { regex: 'def\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\([^)]*\\)\\s*(?:->\\s*[^:]+)?\\s*:', nameGroup: 1, type: 'method' },
                    // Properties with @property decorator
                    { regex: '@property\\s+def\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\(', nameGroup: 1, type: 'getter' },
                ];
            case 'csharp':
                return [
                    // Methods
                    { regex: '(?:public|private|protected|internal)?\\s+(?:static\\s+)?(?:async\\s+)?[\\w<>\\[\\],\\s]+\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\([^)]*\\)\\s*\\{', nameGroup: 1, type: 'method' },
                    // Properties
                    { regex: '(?:public|private|protected|internal)?\\s+(?:static\\s+)?[\\w<>\\[\\],\\s]+\\s+([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\{\\s*(?:get|set)', nameGroup: 1, type: 'property' },
                ];
            default:
                return [
                    // Generic method pattern
                    { regex: '([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\([^)]*\\)\\s*\\{', nameGroup: 1, type: 'method' },
                ];
        }
    }

    /**
     * Extract the body of a member (content between braces)
     */
    private extractMemberBody(source: string, startIndex: number): string {
        let braceCount = 0;
        let inBody = false;
        let bodyStart = -1;

        for (let i = startIndex; i < source.length; i++) {
            const char = source[i];

            if (char === '{') {
                if (!inBody) {
                    bodyStart = i;
                    inBody = true;
                }
                braceCount++;
            } else if (char === '}') {
                braceCount--;
                if (braceCount === 0 && inBody) {
                    return source.slice(bodyStart, i + 1);
                }
            }
        }

        return '';
    }

    /**
     * Extract signature from a member declaration
     */
    private extractSignature(declaration: string, type: MemberType): string {
        // Remove body indicator
        const sig = declaration.replace(/\{$/, '').trim();

        if (type === 'constructor') {
            return 'constructor' + sig.replace('constructor', '');
        }

        return sig;
    }

    /**
     * Check if a member is public
     */
    private isPublicMember(declaration: string, language: string): boolean {
        switch (language) {
            case 'typescript':
            case 'javascript':
            case 'csharp':
                return declaration.includes('public') || !declaration.match(/private|protected/);
            case 'python':
                // Python convention: names starting with _ are private
                const nameMatch = declaration.match(/def\s+([a-zA-Z_][a-zA-Z0-9_]*)/);
                return nameMatch ? !nameMatch[1].startsWith('_') : true;
            default:
                return true;
        }
    }

    /**
     * Count newlines before a position in text
     */
    private countLinesBefore(text: string, position: number): number {
        return (text.slice(0, position).match(/\n/g) || []).length;
    }

    /**
     * Extract source lines
     */
    private extractSourceLines(source: string, startLine: number, endLine: number): string {
        const lines = source.split('\n');
        return lines.slice(startLine - 1, endLine).join('\n');
    }

    /**
     * Diff dependencies (imports)
     */
    private diffDependencies(
        beforeImports: ExtractedImport[],
        afterImports: ExtractedImport[]
    ): DependencyChanges {
        const beforePaths = new Set(beforeImports.map(i => i.importPath));
        const afterPaths = new Set(afterImports.map(i => i.importPath));

        const added: string[] = [];
        const removed: string[] = [];

        for (const path of afterPaths) {
            if (!beforePaths.has(path)) {
                added.push(path);
            }
        }

        for (const path of beforePaths) {
            if (!afterPaths.has(path)) {
                removed.push(path);
            }
        }

        return { added, removed };
    }

    /**
     * Diff inheritance (for classes)
     */
    private diffInheritance(
        beforeSymbol: ExtractedSymbol,
        afterSymbol: ExtractedSymbol
    ): InheritanceChanges | undefined {
        if (beforeSymbol.type !== 'class' && afterSymbol.type !== 'class') {
            return undefined;
        }

        // Extract from signatures
        const beforeExtends = this.extractExtends(beforeSymbol.signature);
        const afterExtends = this.extractExtends(afterSymbol.signature);
        const beforeImplements = this.extractImplements(beforeSymbol.signature);
        const afterImplements = this.extractImplements(afterSymbol.signature);

        // Check if anything changed
        if (beforeExtends === afterExtends &&
            JSON.stringify(beforeImplements) === JSON.stringify(afterImplements)) {
            return undefined;
        }

        return {
            beforeExtends,
            afterExtends,
            beforeImplements,
            afterImplements,
        };
    }

    /**
     * Extract extends clause from signature
     */
    private extractExtends(signature?: string): string | undefined {
        if (!signature) return undefined;
        const match = signature.match(/extends\s+([a-zA-Z_$][a-zA-Z0-9_$<>,\s]*)/);
        return match ? match[1].trim() : undefined;
    }

    /**
     * Extract implements clause from signature
     */
    private extractImplements(signature?: string): string[] | undefined {
        if (!signature) return undefined;
        const match = signature.match(/implements\s+([a-zA-Z_$][a-zA-Z0-9_$<>,\s]*)/);
        if (!match) return undefined;
        return match[1].split(',').map(s => s.trim());
    }

    /**
     * Group symbols by name (only top-level)
     */
    private groupSymbolsByName(symbols: ExtractedSymbol[]): Map<string, ExtractedSymbol> {
        const map = new Map<string, ExtractedSymbol>();

        for (const symbol of symbols) {
            // Only top-level symbols (no parent)
            if (!symbol.parentName) {
                map.set(symbol.name, symbol);
            }
        }

        return map;
    }

    /**
     * Generate a stable node ID
     */
    private generateNodeId(symbol: ExtractedSymbol): string {
        return `${symbol.type}:${symbol.name}`;
    }
}
