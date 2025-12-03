import type { SyntaxNode, Tree } from 'tree-sitter';
import { BaseLanguageAdapter, ExtractedSymbol, ExtractedImport, ExtractedCall } from '../LanguageAdapter';
import type { NodeType } from '../types';

/**
 * Language adapter for TypeScript and JavaScript
 */
export class TypeScriptAdapter extends BaseLanguageAdapter {
    languageId = 'typescript';
    fileExtensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'];
    treeSitterGrammar = 'tree-sitter-typescript';

    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[] {
        const symbols: ExtractedSymbol[] = [];
        const rootNode = tree.rootNode;

        // Extract classes
        const classNodes = this.findNodesOfType(rootNode, ['class_declaration', 'class']);
        for (const node of classNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const isExported = this.isNodeExported(node);

                symbols.push({
                    name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported,
                    signature: this.buildClassSignature(node, sourceCode)
                });

                // Extract methods from class
                const methodNodes = this.findNodesOfType(node, ['method_definition', 'public_field_definition']);
                for (const methodNode of methodNodes) {
                    const methodNameNode = this.findChildByField(methodNode, 'name');
                    if (methodNameNode) {
                        const methodName = this.getNodeText(methodNameNode, sourceCode);
                        symbols.push({
                            name: methodName,
                            type: 'method',
                            lineStart: methodNode.startPosition.row + 1,
                            lineEnd: methodNode.endPosition.row + 1,
                            isExported,
                            parentName: name,
                            signature: this.buildMethodSignature(methodNode, sourceCode)
                        });
                    }
                }
            }
        }

        // Extract interfaces
        const interfaceNodes = this.findNodesOfType(rootNode, ['interface_declaration']);
        for (const node of interfaceNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'interface',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.isNodeExported(node)
                });
            }
        }

        // Extract standalone functions
        const functionNodes = this.findNodesOfType(rootNode, [
            'function_declaration',
            'arrow_function',
            'function_expression'
        ]);

        for (const node of functionNodes) {
            // Skip if inside a class
            if (this.isInsideClass(node)) {
                continue;
            }

            const nameNode = this.findChildByField(node, 'name');
            let name: string;

            if (nameNode) {
                name = this.getNodeText(nameNode, sourceCode);
            } else if (node.parent?.type === 'variable_declarator') {
                // Arrow function assigned to variable
                const varNameNode = this.findChildByField(node.parent, 'name');
                if (varNameNode) {
                    name = this.getNodeText(varNameNode, sourceCode);
                } else {
                    continue;
                }
            } else {
                continue;
            }

            symbols.push({
                name,
                type: 'function',
                lineStart: node.startPosition.row + 1,
                lineEnd: node.endPosition.row + 1,
                isExported: this.isNodeExported(node),
                signature: this.buildFunctionSignature(node, sourceCode)
            });
        }

        // Extract type aliases
        const typeNodes = this.findNodesOfType(rootNode, ['type_alias_declaration']);
        for (const node of typeNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'export',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.isNodeExported(node)
                });
            }
        }

        return symbols;
    }

    extractImports(tree: Tree, sourceCode: string): ExtractedImport[] {
        const imports: ExtractedImport[] = [];
        const rootNode = tree.rootNode;

        const importNodes = this.findNodesOfType(rootNode, ['import_statement']);
        for (const node of importNodes) {
            const sourceNode = this.findChildByField(node, 'source');
            if (!sourceNode) continue;

            const importPath = this.getNodeText(sourceNode, sourceCode)
                .replace(/['"]/g, ''); // Remove quotes

            const importedNames: string[] = [];
            let isDefault = false;
            let isNamespace = false;

            // Check for default import
            const defaultImport = this.findChildOfType(node, 'identifier');
            if (defaultImport && defaultImport.previousSibling?.type !== 'import') {
                importedNames.push(this.getNodeText(defaultImport, sourceCode));
                isDefault = true;
            }

            // Check for namespace import (* as name)
            const namespaceImport = this.findChildOfType(node, 'namespace_import');
            if (namespaceImport) {
                const nameNode = this.findChildOfType(namespaceImport, 'identifier');
                if (nameNode) {
                    importedNames.push(this.getNodeText(nameNode, sourceCode));
                    isNamespace = true;
                }
            }

            // Check for named imports
            const namedImports = this.findChildOfType(node, 'named_imports');
            if (namedImports) {
                const importSpecifiers = this.findNodesOfType(namedImports, ['import_specifier']);
                for (const specifier of importSpecifiers) {
                    const nameNode = this.findChildByField(specifier, 'name') ||
                        this.findChildByField(specifier, 'alias') ||
                        this.findChildOfType(specifier, 'identifier');
                    if (nameNode) {
                        importedNames.push(this.getNodeText(nameNode, sourceCode));
                    }
                }
            }

            imports.push({
                importPath,
                importedNames,
                isDefault,
                isNamespace,
                lineNumber: node.startPosition.row + 1
            });
        }

        // Also handle require() calls
        const callNodes = this.findNodesOfType(rootNode, ['call_expression']);
        for (const node of callNodes) {
            const funcNode = this.findChildByField(node, 'function');
            if (funcNode && this.getNodeText(funcNode, sourceCode) === 'require') {
                const argsNode = this.findChildByField(node, 'arguments');
                if (argsNode) {
                    const stringNode = this.findChildOfType(argsNode, 'string');
                    if (stringNode) {
                        const importPath = this.getNodeText(stringNode, sourceCode)
                            .replace(/['"]/g, '');

                        imports.push({
                            importPath,
                            importedNames: [],
                            isDefault: true,
                            isNamespace: false,
                            lineNumber: node.startPosition.row + 1
                        });
                    }
                }
            }
        }

        return imports;
    }

    extractCalls(tree: Tree, sourceCode: string): ExtractedCall[] {
        const calls: ExtractedCall[] = [];
        const rootNode = tree.rootNode;

        // Find all function/method definitions
        const funcNodes = this.findNodesOfType(rootNode, [
            'function_declaration',
            'arrow_function',
            'function_expression',
            'method_definition'
        ]);

        for (const funcNode of funcNodes) {
            const callerName = this.getFunctionName(funcNode, sourceCode);
            if (!callerName) continue;

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

    private isNodeExported(node: SyntaxNode): boolean {
        // Check if parent is export_statement
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'export_statement') {
                return true;
            }
            // For variable declarations with arrow functions
            if (parent.type === 'lexical_declaration') {
                const grandparent = parent.parent;
                if (grandparent?.type === 'export_statement') {
                    return true;
                }
            }
            parent = parent.parent;
        }
        return false;
    }

    private isInsideClass(node: SyntaxNode): boolean {
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'class_declaration' || parent.type === 'class') {
                return true;
            }
            parent = parent.parent;
        }
        return false;
    }

    private buildClassSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'Anonymous';

        const typeParams = this.findChildOfType(node, 'type_parameters');
        const typeParamsStr = typeParams ? this.getNodeText(typeParams, sourceCode) : '';

        const heritage = this.findChildOfType(node, 'class_heritage');
        const heritageStr = heritage ? ' ' + this.getNodeText(heritage, sourceCode) : '';

        return `class ${name}${typeParamsStr}${heritageStr}`;
    }

    private buildMethodSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'anonymous';

        const params = this.findChildByField(node, 'parameters');
        const paramsStr = params ? this.getNodeText(params, sourceCode) : '()';

        const returnType = this.findChildByField(node, 'return_type');
        const returnStr = returnType ? this.getNodeText(returnType, sourceCode) : '';

        return `${name}${paramsStr}${returnStr}`;
    }

    private buildFunctionSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        let name = nameNode ? this.getNodeText(nameNode, sourceCode) : '';

        // For arrow functions assigned to variables
        if (!name && node.parent?.type === 'variable_declarator') {
            const varNameNode = this.findChildByField(node.parent, 'name');
            if (varNameNode) {
                name = this.getNodeText(varNameNode, sourceCode);
            }
        }

        const params = this.findChildByField(node, 'parameters');
        const paramsStr = params ? this.getNodeText(params, sourceCode) : '()';

        const returnType = this.findChildByField(node, 'return_type');
        const returnStr = returnType ? this.getNodeText(returnType, sourceCode) : '';

        return `function ${name}${paramsStr}${returnStr}`;
    }

    private getFunctionName(node: SyntaxNode, sourceCode: string): string | null {
        const nameNode = this.findChildByField(node, 'name');
        if (nameNode) {
            return this.getNodeText(nameNode, sourceCode);
        }

        // For arrow functions assigned to variables
        if (node.parent?.type === 'variable_declarator') {
            const varNameNode = this.findChildByField(node.parent, 'name');
            if (varNameNode) {
                return this.getNodeText(varNameNode, sourceCode);
            }
        }

        return null;
    }

    private getCalleeName(node: SyntaxNode, sourceCode: string): string | null {
        const funcNode = this.findChildByField(node, 'function');
        if (!funcNode) return null;

        // Simple function call
        if (funcNode.type === 'identifier') {
            return this.getNodeText(funcNode, sourceCode);
        }

        // Method call (obj.method or obj?.method)
        if (funcNode.type === 'member_expression' || funcNode.type === 'optional_chain_expression') {
            const propNode = this.findChildByField(funcNode, 'property');
            if (propNode) {
                return this.getNodeText(propNode, sourceCode);
            }
        }

        return null;
    }
}
