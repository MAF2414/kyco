import type { SyntaxNode, Tree } from 'tree-sitter';
import { BaseLanguageAdapter, ExtractedSymbol, ExtractedImport, ExtractedCall } from '../LanguageAdapter';

/**
 * Language adapter for Go
 */
export class GoAdapter extends BaseLanguageAdapter {
    languageId = 'go';
    fileExtensions = ['.go'];
    treeSitterGrammar = 'tree-sitter-go';

    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[] {
        const symbols: ExtractedSymbol[] = [];
        const rootNode = tree.rootNode;

        // Extract type declarations (structs, interfaces, type aliases)
        const typeNodes = this.findNodesOfType(rootNode, ['type_declaration']);
        for (const node of typeNodes) {
            const specNodes = this.findNodesOfType(node, ['type_spec']);
            for (const specNode of specNodes) {
                const nameNode = this.findChildByField(specNode, 'name');
                if (!nameNode) continue;

                const name = this.getNodeText(nameNode, sourceCode);
                const isExported = this.isExportedName(name);

                const typeNode = this.findChildByField(specNode, 'type');
                if (!typeNode) continue;

                if (typeNode.type === 'struct_type') {
                    symbols.push({
                        name,
                        type: 'class',
                        lineStart: specNode.startPosition.row + 1,
                        lineEnd: specNode.endPosition.row + 1,
                        isExported,
                        signature: `type ${name} struct`
                    });
                } else if (typeNode.type === 'interface_type') {
                    symbols.push({
                        name,
                        type: 'interface',
                        lineStart: specNode.startPosition.row + 1,
                        lineEnd: specNode.endPosition.row + 1,
                        isExported,
                        signature: `type ${name} interface`
                    });
                } else {
                    // Type alias or other type definition
                    symbols.push({
                        name,
                        type: 'export',
                        lineStart: specNode.startPosition.row + 1,
                        lineEnd: specNode.endPosition.row + 1,
                        isExported,
                        signature: `type ${name} ${this.getNodeText(typeNode, sourceCode).split('\n')[0]}`
                    });
                }
            }
        }

        // Extract function declarations
        const functionNodes = this.findNodesOfType(rootNode, ['function_declaration']);
        for (const node of functionNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'function',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.isExportedName(name),
                    signature: this.buildFunctionSignature(node, sourceCode)
                });
            }
        }

        // Extract method declarations (functions with receivers)
        const methodNodes = this.findNodesOfType(rootNode, ['method_declaration']);
        for (const node of methodNodes) {
            const nameNode = this.findChildByField(node, 'name');
            const receiverNode = this.findChildByField(node, 'receiver');

            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const receiverType = this.getReceiverType(receiverNode, sourceCode);

                symbols.push({
                    name,
                    type: 'method',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.isExportedName(name),
                    parentName: receiverType ?? undefined,
                    signature: this.buildMethodSignature(node, sourceCode)
                });
            }
        }

        return symbols;
    }

    extractImports(tree: Tree, sourceCode: string): ExtractedImport[] {
        const imports: ExtractedImport[] = [];
        const rootNode = tree.rootNode;

        // Handle import declarations
        const importDeclNodes = this.findNodesOfType(rootNode, ['import_declaration']);
        for (const node of importDeclNodes) {
            // Single import
            const importSpecNodes = this.findNodesOfType(node, ['import_spec']);
            for (const specNode of importSpecNodes) {
                const pathNode = this.findChildByField(specNode, 'path');
                if (!pathNode) continue;

                const importPath = this.getNodeText(pathNode, sourceCode).replace(/"/g, '');
                const nameNode = this.findChildByField(specNode, 'name');

                let alias: string | undefined;
                let isNamespace = false;

                if (nameNode) {
                    const aliasText = this.getNodeText(nameNode, sourceCode);
                    if (aliasText === '.') {
                        isNamespace = true;
                    } else if (aliasText !== '_') {
                        alias = aliasText;
                    }
                }

                // Get package name from path (last segment)
                const packageName = alias || importPath.split('/').pop() || importPath;

                imports.push({
                    importPath,
                    importedNames: [packageName],
                    isDefault: false,
                    isNamespace,
                    lineNumber: specNode.startPosition.row + 1
                });
            }
        }

        return imports;
    }

    extractCalls(tree: Tree, sourceCode: string): ExtractedCall[] {
        const calls: ExtractedCall[] = [];
        const rootNode = tree.rootNode;

        // Find all function/method declarations
        const funcNodes = this.findNodesOfType(rootNode, ['function_declaration', 'method_declaration']);
        for (const funcNode of funcNodes) {
            const callerNameNode = this.findChildByField(funcNode, 'name');
            if (!callerNameNode) continue;

            const callerName = this.getNodeText(callerNameNode, sourceCode);

            // Find all call expressions within this function
            const callNodes = this.findNodesOfType(funcNode, ['call_expression']);
            for (const callNode of callNodes) {
                const calleeName = this.getCalleeName(callNode, sourceCode);
                if (calleeName) {
                    calls.push({
                        callerName,
                        calleeName,
                        lineNumber: callNode.startPosition.row + 1
                    });
                }
            }
        }

        return calls;
    }

    /**
     * In Go, exported names start with an uppercase letter
     */
    private isExportedName(name: string): boolean {
        return name.length > 0 && name[0] === name[0].toUpperCase() && name[0] !== name[0].toLowerCase();
    }

    private getReceiverType(receiverNode: SyntaxNode | null, sourceCode: string): string | null {
        if (!receiverNode) return null;

        // Look for the type in the receiver parameter list
        const paramNodes = this.findNodesOfType(receiverNode, ['parameter_declaration']);
        for (const paramNode of paramNodes) {
            const typeNode = this.findChildByField(paramNode, 'type');
            if (typeNode) {
                let typeName = this.getNodeText(typeNode, sourceCode);
                // Remove pointer indicator if present
                typeName = typeName.replace(/^\*/, '');
                return typeName;
            }
        }

        return null;
    }

    private buildFunctionSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'unknown';

        const paramsNode = this.findChildByField(node, 'parameters');
        const params = paramsNode ? this.getNodeText(paramsNode, sourceCode) : '()';

        const resultNode = this.findChildByField(node, 'result');
        const result = resultNode ? ' ' + this.getNodeText(resultNode, sourceCode) : '';

        return `func ${name}${params}${result}`;
    }

    private buildMethodSignature(node: SyntaxNode, sourceCode: string): string {
        const receiverNode = this.findChildByField(node, 'receiver');
        const receiver = receiverNode ? this.getNodeText(receiverNode, sourceCode) + ' ' : '';

        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'unknown';

        const paramsNode = this.findChildByField(node, 'parameters');
        const params = paramsNode ? this.getNodeText(paramsNode, sourceCode) : '()';

        const resultNode = this.findChildByField(node, 'result');
        const result = resultNode ? ' ' + this.getNodeText(resultNode, sourceCode) : '';

        return `func ${receiver}${name}${params}${result}`;
    }

    private getCalleeName(node: SyntaxNode, sourceCode: string): string | null {
        const funcNode = this.findChildByField(node, 'function');
        if (!funcNode) return null;

        // Simple function call
        if (funcNode.type === 'identifier') {
            return this.getNodeText(funcNode, sourceCode);
        }

        // Selector expression (pkg.Func or obj.Method)
        if (funcNode.type === 'selector_expression') {
            const fieldNode = this.findChildByField(funcNode, 'field');
            if (fieldNode) {
                return this.getNodeText(fieldNode, sourceCode);
            }
        }

        return null;
    }
}
