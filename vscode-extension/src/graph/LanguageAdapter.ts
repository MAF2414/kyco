import type { SyntaxNode, Tree } from 'tree-sitter';
import type { NodeType } from './types';

/**
 * Represents an extracted symbol from the AST
 */
export interface ExtractedSymbol {
    name: string;
    type: NodeType;
    lineStart: number;
    lineEnd: number;
    isExported: boolean;
    signature?: string;
    parentName?: string;
}

/**
 * Represents an import relationship
 */
export interface ExtractedImport {
    importPath: string;
    importedNames: string[];
    isDefault: boolean;
    isNamespace: boolean;
    lineNumber: number;
}

/**
 * Represents a call relationship
 */
export interface ExtractedCall {
    callerName: string;
    calleeName: string;
    lineNumber: number;
}

/**
 * Interface for language-specific AST parsing logic.
 * Each supported language implements this interface.
 */
export interface LanguageAdapter {
    /** Unique language identifier (e.g., 'typescript', 'python') */
    languageId: string;

    /** File extensions this adapter handles */
    fileExtensions: string[];

    /** Tree-sitter grammar module name */
    treeSitterGrammar: string;

    /**
     * Extract all symbols (classes, functions, etc.) from the AST
     */
    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[];

    /**
     * Extract all import statements from the AST
     */
    extractImports(tree: Tree, sourceCode: string): ExtractedImport[];

    /**
     * Extract call relationships from the AST
     */
    extractCalls(tree: Tree, sourceCode: string): ExtractedCall[];

    /**
     * Get the text content of a node
     */
    getNodeText(node: SyntaxNode, sourceCode: string): string;
}

/**
 * Base class with common utilities for language adapters
 */
export abstract class BaseLanguageAdapter implements LanguageAdapter {
    abstract languageId: string;
    abstract fileExtensions: string[];
    abstract treeSitterGrammar: string;

    abstract extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[];
    abstract extractImports(tree: Tree, sourceCode: string): ExtractedImport[];
    abstract extractCalls(tree: Tree, sourceCode: string): ExtractedCall[];

    getNodeText(node: SyntaxNode, sourceCode: string): string {
        return sourceCode.slice(node.startIndex, node.endIndex);
    }

    /**
     * Find all nodes of specific types in the tree
     */
    protected findNodesOfType(node: SyntaxNode, types: string[]): SyntaxNode[] {
        const results: SyntaxNode[] = [];

        const traverse = (n: SyntaxNode) => {
            if (types.includes(n.type)) {
                results.push(n);
            }
            for (let i = 0; i < n.childCount; i++) {
                const child = n.child(i);
                if (child) {
                    traverse(child);
                }
            }
        };

        traverse(node);
        return results;
    }

    /**
     * Find first child of a specific type
     */
    protected findChildOfType(node: SyntaxNode, type: string): SyntaxNode | null {
        for (let i = 0; i < node.childCount; i++) {
            const child = node.child(i);
            if (child && child.type === type) {
                return child;
            }
        }
        return null;
    }

    /**
     * Find child by field name
     */
    protected findChildByField(node: SyntaxNode, fieldName: string): SyntaxNode | null {
        return node.childForFieldName(fieldName);
    }

    /**
     * Check if a node has an export modifier
     */
    protected hasExportModifier(node: SyntaxNode): boolean {
        // Check previous siblings for export keyword
        let sibling = node.previousSibling;
        while (sibling) {
            if (sibling.type === 'export_statement' || sibling.type === 'export') {
                return true;
            }
            sibling = sibling.previousSibling;
        }

        // Check if node is child of export_statement
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'export_statement') {
                return true;
            }
            parent = parent.parent;
        }

        return false;
    }

    /**
     * Build a function/method signature from its AST node
     */
    protected buildSignature(
        name: string,
        parameters: string,
        returnType?: string
    ): string {
        let sig = `${name}(${parameters})`;
        if (returnType) {
            sig += `: ${returnType}`;
        }
        return sig;
    }
}
