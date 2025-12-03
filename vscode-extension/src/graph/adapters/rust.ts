import type { SyntaxNode, Tree } from 'tree-sitter';
import { BaseLanguageAdapter, ExtractedSymbol, ExtractedImport, ExtractedCall } from '../LanguageAdapter';

/**
 * Language adapter for Rust
 */
export class RustAdapter extends BaseLanguageAdapter {
    languageId = 'rust';
    fileExtensions = ['.rs'];
    treeSitterGrammar = 'tree-sitter-rust';

    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[] {
        const symbols: ExtractedSymbol[] = [];
        const rootNode = tree.rootNode;

        // Extract structs
        const structNodes = this.findNodesOfType(rootNode, ['struct_item']);
        for (const node of structNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const isExported = this.hasPubModifier(node);

                symbols.push({
                    name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported,
                    signature: this.buildStructSignature(node, sourceCode)
                });
            }
        }

        // Extract enums
        const enumNodes = this.findNodesOfType(rootNode, ['enum_item']);
        for (const node of enumNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPubModifier(node),
                    signature: `enum ${name}`
                });
            }
        }

        // Extract traits
        const traitNodes = this.findNodesOfType(rootNode, ['trait_item']);
        for (const node of traitNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'interface',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPubModifier(node),
                    signature: `trait ${name}`
                });
            }
        }

        // Extract impl blocks and their methods
        const implNodes = this.findNodesOfType(rootNode, ['impl_item']);
        for (const node of implNodes) {
            const typeNode = this.findChildByField(node, 'type');
            if (!typeNode) continue;

            const typeName = this.getNodeText(typeNode, sourceCode);

            // Extract functions from impl block
            const fnNodes = this.findNodesOfType(node, ['function_item']);
            for (const fnNode of fnNodes) {
                const fnNameNode = this.findChildByField(fnNode, 'name');
                if (fnNameNode) {
                    const fnName = this.getNodeText(fnNameNode, sourceCode);
                    symbols.push({
                        name: fnName,
                        type: 'method',
                        lineStart: fnNode.startPosition.row + 1,
                        lineEnd: fnNode.endPosition.row + 1,
                        isExported: this.hasPubModifier(fnNode),
                        parentName: typeName,
                        signature: this.buildFunctionSignature(fnNode, sourceCode)
                    });
                }
            }
        }

        // Extract standalone functions (not in impl blocks)
        const functionNodes = this.findNodesOfType(rootNode, ['function_item']);
        for (const node of functionNodes) {
            // Skip if inside impl block
            if (this.isInsideImpl(node)) continue;

            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'function',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPubModifier(node),
                    signature: this.buildFunctionSignature(node, sourceCode)
                });
            }
        }

        // Extract modules
        const modNodes = this.findNodesOfType(rootNode, ['mod_item']);
        for (const node of modNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                symbols.push({
                    name,
                    type: 'namespace',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPubModifier(node)
                });
            }
        }

        return symbols;
    }

    extractImports(tree: Tree, sourceCode: string): ExtractedImport[] {
        const imports: ExtractedImport[] = [];
        const rootNode = tree.rootNode;

        // Handle use statements
        const useNodes = this.findNodesOfType(rootNode, ['use_declaration']);
        for (const node of useNodes) {
            const pathNode = this.findChildOfType(node, 'use_tree') ||
                this.findChildOfType(node, 'scoped_identifier') ||
                this.findChildOfType(node, 'identifier');

            if (pathNode) {
                const importPath = this.getNodeText(pathNode, sourceCode);
                const parsedImport = this.parseUseStatement(importPath);

                imports.push({
                    importPath: parsedImport.path,
                    importedNames: parsedImport.names,
                    isDefault: false,
                    isNamespace: parsedImport.isGlob,
                    lineNumber: node.startPosition.row + 1
                });
            }
        }

        // Handle extern crate declarations
        const externNodes = this.findNodesOfType(rootNode, ['extern_crate_declaration']);
        for (const node of externNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const crateName = this.getNodeText(nameNode, sourceCode);
                imports.push({
                    importPath: crateName,
                    importedNames: [crateName],
                    isDefault: false,
                    isNamespace: true,
                    lineNumber: node.startPosition.row + 1
                });
            }
        }

        return imports;
    }

    extractCalls(tree: Tree, sourceCode: string): ExtractedCall[] {
        const calls: ExtractedCall[] = [];
        const rootNode = tree.rootNode;

        // Find all function items
        const funcNodes = this.findNodesOfType(rootNode, ['function_item']);
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

            // Handle method calls
            const methodCallNodes = this.findNodesOfType(funcNode, ['method_call_expression']);
            for (const callNode of methodCallNodes) {
                const methodNameNode = this.findChildByField(callNode, 'name');
                if (methodNameNode) {
                    calls.push({
                        callerName,
                        calleeName: this.getNodeText(methodNameNode, sourceCode),
                        lineNumber: callNode.startPosition.row + 1
                    });
                }
            }
        }

        return calls;
    }

    private hasPubModifier(node: SyntaxNode): boolean {
        // Check for visibility modifier
        for (let i = 0; i < node.childCount; i++) {
            const child = node.child(i);
            if (child && child.type === 'visibility_modifier') {
                return true;
            }
        }
        return false;
    }

    private isInsideImpl(node: SyntaxNode): boolean {
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'impl_item') {
                return true;
            }
            parent = parent.parent;
        }
        return false;
    }

    private buildStructSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'Unknown';

        const typeParamsNode = this.findChildOfType(node, 'type_parameters');
        const typeParams = typeParamsNode ? this.getNodeText(typeParamsNode, sourceCode) : '';

        return `struct ${name}${typeParams}`;
    }

    private buildFunctionSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'unknown';

        const paramsNode = this.findChildByField(node, 'parameters');
        const params = paramsNode ? this.getNodeText(paramsNode, sourceCode) : '()';

        const returnTypeNode = this.findChildByField(node, 'return_type');
        const returnType = returnTypeNode ? ' ' + this.getNodeText(returnTypeNode, sourceCode) : '';

        return `fn ${name}${params}${returnType}`;
    }

    private parseUseStatement(useTree: string): { path: string; names: string[]; isGlob: boolean } {
        // Handle glob imports: use foo::*
        if (useTree.endsWith('*')) {
            return {
                path: useTree.replace('::*', ''),
                names: ['*'],
                isGlob: true
            };
        }

        // Handle grouped imports: use foo::{bar, baz}
        const groupMatch = useTree.match(/^(.+)::\{(.+)\}$/);
        if (groupMatch) {
            const basePath = groupMatch[1];
            const names = groupMatch[2].split(',').map(n => n.trim());
            return {
                path: basePath,
                names,
                isGlob: false
            };
        }

        // Handle aliased imports: use foo as bar
        const aliasMatch = useTree.match(/^(.+)\s+as\s+(\w+)$/);
        if (aliasMatch) {
            return {
                path: aliasMatch[1],
                names: [aliasMatch[2]],
                isGlob: false
            };
        }

        // Simple import
        const parts = useTree.split('::');
        const name = parts[parts.length - 1];
        return {
            path: useTree,
            names: [name],
            isGlob: false
        };
    }

    private getCalleeName(node: SyntaxNode, sourceCode: string): string | null {
        const funcNode = this.findChildByField(node, 'function');
        if (!funcNode) return null;

        if (funcNode.type === 'identifier') {
            return this.getNodeText(funcNode, sourceCode);
        }

        // Scoped identifier (module::function)
        if (funcNode.type === 'scoped_identifier') {
            const nameNode = this.findChildByField(funcNode, 'name');
            if (nameNode) {
                return this.getNodeText(nameNode, sourceCode);
            }
        }

        // Field expression (obj.field, though usually method calls)
        if (funcNode.type === 'field_expression') {
            const fieldNode = this.findChildByField(funcNode, 'field');
            if (fieldNode) {
                return this.getNodeText(fieldNode, sourceCode);
            }
        }

        return null;
    }
}
