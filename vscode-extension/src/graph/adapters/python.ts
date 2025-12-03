import type { SyntaxNode, Tree } from 'tree-sitter';
import { BaseLanguageAdapter, ExtractedSymbol, ExtractedImport, ExtractedCall } from '../LanguageAdapter';

/**
 * Language adapter for Python
 */
export class PythonAdapter extends BaseLanguageAdapter {
    languageId = 'python';
    fileExtensions = ['.py', '.pyi'];
    treeSitterGrammar = 'tree-sitter-python';

    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[] {
        const symbols: ExtractedSymbol[] = [];
        const rootNode = tree.rootNode;

        // Extract classes
        const classNodes = this.findNodesOfType(rootNode, ['class_definition']);
        for (const node of classNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const isExported = !name.startsWith('_');

                symbols.push({
                    name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported,
                    signature: this.buildClassSignature(node, sourceCode)
                });

                // Extract methods from class
                const methodNodes = this.findNodesOfType(node, ['function_definition']);
                for (const methodNode of methodNodes) {
                    // Only direct children (methods), not nested functions
                    if (this.getParentClass(methodNode) !== node) continue;

                    const methodNameNode = this.findChildByField(methodNode, 'name');
                    if (methodNameNode) {
                        const methodName = this.getNodeText(methodNameNode, sourceCode);
                        const isMethodExported = !methodName.startsWith('_') || methodName.startsWith('__') && methodName.endsWith('__');

                        symbols.push({
                            name: methodName,
                            type: 'method',
                            lineStart: methodNode.startPosition.row + 1,
                            lineEnd: methodNode.endPosition.row + 1,
                            isExported: isMethodExported,
                            parentName: name,
                            signature: this.buildFunctionSignature(methodNode, sourceCode)
                        });
                    }
                }
            }
        }

        // Extract standalone functions (not inside classes)
        const functionNodes = this.findNodesOfType(rootNode, ['function_definition']);
        for (const node of functionNodes) {
            // Skip methods inside classes
            if (this.getParentClass(node)) continue;

            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'function',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: !name.startsWith('_'),
                    signature: this.buildFunctionSignature(node, sourceCode)
                });
            }
        }

        return symbols;
    }

    extractImports(tree: Tree, sourceCode: string): ExtractedImport[] {
        const imports: ExtractedImport[] = [];
        const rootNode = tree.rootNode;

        // Handle "import x" statements
        const importNodes = this.findNodesOfType(rootNode, ['import_statement']);
        for (const node of importNodes) {
            const nameNode = this.findChildOfType(node, 'dotted_name');
            if (nameNode) {
                const importPath = this.getNodeText(nameNode, sourceCode);
                imports.push({
                    importPath,
                    importedNames: [importPath.split('.').pop() || importPath],
                    isDefault: false,
                    isNamespace: true,
                    lineNumber: node.startPosition.row + 1
                });
            }

            // Handle aliased imports: import x as y
            const aliasedNodes = this.findNodesOfType(node, ['aliased_import']);
            for (const aliased of aliasedNodes) {
                const moduleName = this.findChildByField(aliased, 'name');
                const aliasName = this.findChildByField(aliased, 'alias');
                if (moduleName) {
                    const importPath = this.getNodeText(moduleName, sourceCode);
                    const alias = aliasName ? this.getNodeText(aliasName, sourceCode) : undefined;
                    imports.push({
                        importPath,
                        importedNames: [alias || importPath],
                        isDefault: false,
                        isNamespace: true,
                        lineNumber: node.startPosition.row + 1
                    });
                }
            }
        }

        // Handle "from x import y" statements
        const fromImportNodes = this.findNodesOfType(rootNode, ['import_from_statement']);
        for (const node of fromImportNodes) {
            const moduleNode = this.findChildByField(node, 'module_name');
            if (!moduleNode) continue;

            const importPath = this.getNodeText(moduleNode, sourceCode);
            const importedNames: string[] = [];
            let isNamespace = false;

            // Check for wildcard import: from x import *
            const wildcardNode = this.findChildOfType(node, 'wildcard_import');
            if (wildcardNode) {
                isNamespace = true;
                importedNames.push('*');
            } else {
                // Named imports
                const nameNodes = this.findNodesOfType(node, ['dotted_name', 'aliased_import']);
                for (const nameNode of nameNodes) {
                    if (nameNode === moduleNode) continue;

                    if (nameNode.type === 'aliased_import') {
                        const aliasNode = this.findChildByField(nameNode, 'alias');
                        const origNode = this.findChildByField(nameNode, 'name');
                        if (aliasNode) {
                            importedNames.push(this.getNodeText(aliasNode, sourceCode));
                        } else if (origNode) {
                            importedNames.push(this.getNodeText(origNode, sourceCode));
                        }
                    } else {
                        importedNames.push(this.getNodeText(nameNode, sourceCode));
                    }
                }
            }

            imports.push({
                importPath,
                importedNames,
                isDefault: false,
                isNamespace,
                lineNumber: node.startPosition.row + 1
            });
        }

        return imports;
    }

    extractCalls(tree: Tree, sourceCode: string): ExtractedCall[] {
        const calls: ExtractedCall[] = [];
        const rootNode = tree.rootNode;

        // Find all function definitions
        const funcNodes = this.findNodesOfType(rootNode, ['function_definition']);
        for (const funcNode of funcNodes) {
            const callerNameNode = this.findChildByField(funcNode, 'name');
            if (!callerNameNode) continue;

            const callerName = this.getNodeText(callerNameNode, sourceCode);

            // Find all call expressions within this function
            const callNodes = this.findNodesOfType(funcNode, ['call']);
            for (const callNode of callNodes) {
                const funcCallNode = this.findChildByField(callNode, 'function');
                if (!funcCallNode) continue;

                const calleeName = this.getCalleeName(funcCallNode, sourceCode);
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

    private getParentClass(node: SyntaxNode): SyntaxNode | null {
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'class_definition') {
                return parent;
            }
            parent = parent.parent;
        }
        return null;
    }

    private buildClassSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'Unknown';

        const argsNode = this.findChildByField(node, 'superclasses');
        const superclasses = argsNode ? this.getNodeText(argsNode, sourceCode) : '';

        return `class ${name}${superclasses}`;
    }

    private buildFunctionSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'unknown';

        const paramsNode = this.findChildByField(node, 'parameters');
        const params = paramsNode ? this.getNodeText(paramsNode, sourceCode) : '()';

        const returnTypeNode = this.findChildByField(node, 'return_type');
        const returnType = returnTypeNode ? ` -> ${this.getNodeText(returnTypeNode, sourceCode)}` : '';

        return `def ${name}${params}${returnType}`;
    }

    private getCalleeName(node: SyntaxNode, sourceCode: string): string | null {
        if (node.type === 'identifier') {
            return this.getNodeText(node, sourceCode);
        }

        if (node.type === 'attribute') {
            const attrNode = this.findChildByField(node, 'attribute');
            if (attrNode) {
                return this.getNodeText(attrNode, sourceCode);
            }
        }

        return null;
    }
}
