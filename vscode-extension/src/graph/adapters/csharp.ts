import type { SyntaxNode, Tree } from 'tree-sitter';
import { BaseLanguageAdapter, ExtractedSymbol, ExtractedImport, ExtractedCall } from '../LanguageAdapter';

/**
 * Language adapter for C#
 */
export class CSharpAdapter extends BaseLanguageAdapter {
    languageId = 'csharp';
    fileExtensions = ['.cs'];
    treeSitterGrammar = 'tree-sitter-c-sharp';

    extractSymbols(tree: Tree, sourceCode: string): ExtractedSymbol[] {
        const symbols: ExtractedSymbol[] = [];
        const rootNode = tree.rootNode;

        // Extract classes
        const classNodes = this.findNodesOfType(rootNode, ['class_declaration']);
        for (const node of classNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const isExported = this.hasPublicModifier(node);
                const namespace = this.getNamespace(node);

                symbols.push({
                    name: namespace ? `${namespace}.${name}` : name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported,
                    signature: this.buildClassSignature(node, sourceCode)
                });

                // Extract methods from class
                const methodNodes = this.findNodesOfType(node, ['method_declaration']);
                for (const methodNode of methodNodes) {
                    const methodNameNode = this.findChildByField(methodNode, 'name');
                    if (methodNameNode) {
                        const methodName = this.getNodeText(methodNameNode, sourceCode);
                        symbols.push({
                            name: methodName,
                            type: 'method',
                            lineStart: methodNode.startPosition.row + 1,
                            lineEnd: methodNode.endPosition.row + 1,
                            isExported: this.hasPublicModifier(methodNode),
                            parentName: name,
                            signature: this.buildMethodSignature(methodNode, sourceCode)
                        });
                    }
                }

                // Extract properties
                const propertyNodes = this.findNodesOfType(node, ['property_declaration']);
                for (const propNode of propertyNodes) {
                    const propNameNode = this.findChildByField(propNode, 'name');
                    if (propNameNode) {
                        const propName = this.getNodeText(propNameNode, sourceCode);
                        symbols.push({
                            name: propName,
                            type: 'method', // Treating properties as methods for visualization
                            lineStart: propNode.startPosition.row + 1,
                            lineEnd: propNode.endPosition.row + 1,
                            isExported: this.hasPublicModifier(propNode),
                            parentName: name
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
                const namespace = this.getNamespace(node);

                symbols.push({
                    name: namespace ? `${namespace}.${name}` : name,
                    type: 'interface',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPublicModifier(node)
                });
            }
        }

        // Extract structs
        const structNodes = this.findNodesOfType(rootNode, ['struct_declaration']);
        for (const node of structNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const namespace = this.getNamespace(node);

                symbols.push({
                    name: namespace ? `${namespace}.${name}` : name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPublicModifier(node),
                    signature: `struct ${name}`
                });
            }
        }

        // Extract enums
        const enumNodes = this.findNodesOfType(rootNode, ['enum_declaration']);
        for (const node of enumNodes) {
            const nameNode = this.findChildByField(node, 'name');
            if (nameNode) {
                const name = this.getNodeText(nameNode, sourceCode);
                const namespace = this.getNamespace(node);

                symbols.push({
                    name: namespace ? `${namespace}.${name}` : name,
                    type: 'class',
                    lineStart: node.startPosition.row + 1,
                    lineEnd: node.endPosition.row + 1,
                    isExported: this.hasPublicModifier(node),
                    signature: `enum ${name}`
                });
            }
        }

        return symbols;
    }

    extractImports(tree: Tree, sourceCode: string): ExtractedImport[] {
        const imports: ExtractedImport[] = [];
        const rootNode = tree.rootNode;

        // Handle using directives
        const usingNodes = this.findNodesOfType(rootNode, ['using_directive']);
        for (const node of usingNodes) {
            const nameNode = this.findChildOfType(node, 'qualified_name') ||
                this.findChildOfType(node, 'identifier');

            if (nameNode) {
                const importPath = this.getNodeText(nameNode, sourceCode);

                // Check for alias (using X = Y)
                const aliasNode = this.findChildOfType(node, 'name_equals');
                const alias = aliasNode ? this.getNodeText(aliasNode, sourceCode).replace('=', '').trim() : undefined;

                imports.push({
                    importPath,
                    importedNames: alias ? [alias] : [importPath.split('.').pop() || importPath],
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

        // Find all method declarations
        const methodNodes = this.findNodesOfType(rootNode, ['method_declaration', 'constructor_declaration']);
        for (const methodNode of methodNodes) {
            const callerNameNode = this.findChildByField(methodNode, 'name');
            if (!callerNameNode) continue;

            const callerName = this.getNodeText(callerNameNode, sourceCode);

            // Find all invocation expressions within this method
            const invocationNodes = this.findNodesOfType(methodNode, ['invocation_expression']);
            for (const invNode of invocationNodes) {
                const calleeName = this.getCalleeName(invNode, sourceCode);
                if (calleeName) {
                    calls.push({
                        callerName,
                        calleeName,
                        lineNumber: invNode.startPosition.row + 1
                    });
                }
            }

            // Also handle object creation (new X())
            const objectCreationNodes = this.findNodesOfType(methodNode, ['object_creation_expression']);
            for (const objNode of objectCreationNodes) {
                const typeNode = this.findChildByField(objNode, 'type');
                if (typeNode) {
                    const typeName = this.getNodeText(typeNode, sourceCode);
                    calls.push({
                        callerName,
                        calleeName: typeName,
                        lineNumber: objNode.startPosition.row + 1
                    });
                }
            }
        }

        return calls;
    }

    private hasPublicModifier(node: SyntaxNode): boolean {
        // Check for modifier nodes
        let sibling = node.previousSibling;
        while (sibling) {
            if (sibling.type === 'modifier' || sibling.type === 'modifiers') {
                const text = sibling.text;
                if (text.includes('public') || text.includes('internal')) {
                    return true;
                }
            }
            sibling = sibling.previousSibling;
        }

        // Also check first child for modifiers
        for (let i = 0; i < node.childCount; i++) {
            const child = node.child(i);
            if (child && (child.type === 'modifier' || child.type === 'modifiers')) {
                const text = child.text;
                if (text.includes('public') || text.includes('internal')) {
                    return true;
                }
            }
        }

        return false;
    }

    private getNamespace(node: SyntaxNode): string | null {
        let parent = node.parent;
        while (parent) {
            if (parent.type === 'namespace_declaration') {
                const nameNode = this.findChildByField(parent, 'name') ||
                    this.findChildOfType(parent, 'qualified_name');
                if (nameNode) {
                    return nameNode.text;
                }
            }
            parent = parent.parent;
        }
        return null;
    }

    private buildClassSignature(node: SyntaxNode, sourceCode: string): string {
        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'Unknown';

        const baseListNode = this.findChildOfType(node, 'base_list');
        const baseList = baseListNode ? ' : ' + this.getNodeText(baseListNode, sourceCode) : '';

        const typeParamsNode = this.findChildOfType(node, 'type_parameter_list');
        const typeParams = typeParamsNode ? this.getNodeText(typeParamsNode, sourceCode) : '';

        return `class ${name}${typeParams}${baseList}`;
    }

    private buildMethodSignature(node: SyntaxNode, sourceCode: string): string {
        const returnTypeNode = this.findChildByField(node, 'type');
        const returnType = returnTypeNode ? this.getNodeText(returnTypeNode, sourceCode) + ' ' : '';

        const nameNode = this.findChildByField(node, 'name');
        const name = nameNode ? this.getNodeText(nameNode, sourceCode) : 'unknown';

        const paramsNode = this.findChildByField(node, 'parameters');
        const params = paramsNode ? this.getNodeText(paramsNode, sourceCode) : '()';

        return `${returnType}${name}${params}`;
    }

    private getCalleeName(node: SyntaxNode, sourceCode: string): string | null {
        // Direct function call
        const funcNode = node.child(0);
        if (!funcNode) return null;

        if (funcNode.type === 'identifier') {
            return this.getNodeText(funcNode, sourceCode);
        }

        // Member access (obj.Method())
        if (funcNode.type === 'member_access_expression') {
            const memberNode = this.findChildByField(funcNode, 'name');
            if (memberNode) {
                return this.getNodeText(memberNode, sourceCode);
            }
        }

        // Generic method call (Method<T>())
        if (funcNode.type === 'generic_name') {
            const identNode = this.findChildOfType(funcNode, 'identifier');
            if (identNode) {
                return this.getNodeText(identNode, sourceCode);
            }
        }

        return null;
    }
}
