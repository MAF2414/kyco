/**
 * Type definitions for tree-sitter
 */
declare module 'tree-sitter' {
    export interface Point {
        row: number;
        column: number;
    }

    export interface Range {
        startPosition: Point;
        endPosition: Point;
        startIndex: number;
        endIndex: number;
    }

    export interface Edit {
        startIndex: number;
        oldEndIndex: number;
        newEndIndex: number;
        startPosition: Point;
        oldEndPosition: Point;
        newEndPosition: Point;
    }

    export interface SyntaxNode {
        id: number;
        tree: Tree;
        type: string;
        text: string;
        startPosition: Point;
        endPosition: Point;
        startIndex: number;
        endIndex: number;
        parent: SyntaxNode | null;
        children: SyntaxNode[];
        childCount: number;
        namedChildren: SyntaxNode[];
        namedChildCount: number;
        firstChild: SyntaxNode | null;
        firstNamedChild: SyntaxNode | null;
        lastChild: SyntaxNode | null;
        lastNamedChild: SyntaxNode | null;
        nextSibling: SyntaxNode | null;
        nextNamedSibling: SyntaxNode | null;
        previousSibling: SyntaxNode | null;
        previousNamedSibling: SyntaxNode | null;
        hasError: boolean;
        isMissing: boolean;

        child(index: number): SyntaxNode | null;
        namedChild(index: number): SyntaxNode | null;
        childForFieldName(fieldName: string): SyntaxNode | null;
        childrenForFieldName(fieldName: string): SyntaxNode[];
        firstChildForIndex(index: number): SyntaxNode | null;
        firstNamedChildForIndex(index: number): SyntaxNode | null;
        descendantForIndex(startIndex: number, endIndex?: number): SyntaxNode;
        descendantForPosition(startPosition: Point, endPosition?: Point): SyntaxNode;
        namedDescendantForIndex(startIndex: number, endIndex?: number): SyntaxNode;
        namedDescendantForPosition(startPosition: Point, endPosition?: Point): SyntaxNode;
        descendantsOfType(types: string | string[], startPosition?: Point, endPosition?: Point): SyntaxNode[];
        closest(types: string | string[]): SyntaxNode | null;
        walk(): TreeCursor;
        toString(): string;
    }

    export interface TreeCursor {
        nodeType: string;
        nodeText: string;
        nodeId: number;
        nodeIsNamed: boolean;
        nodeIsMissing: boolean;
        startPosition: Point;
        endPosition: Point;
        startIndex: number;
        endIndex: number;
        currentNode: SyntaxNode;
        currentFieldName: string | null;

        gotoParent(): boolean;
        gotoFirstChild(): boolean;
        gotoFirstChildForIndex(index: number): boolean;
        gotoFirstChildForPosition(position: Point): boolean;
        gotoNextSibling(): boolean;
        gotoPreviousSibling(): boolean;
        gotoDescendant(goalDescendantIndex: number): void;
        reset(node: SyntaxNode): void;
        resetTo(cursor: TreeCursor): void;
    }

    export interface Tree {
        rootNode: SyntaxNode;
        language: Language;

        edit(edit: Edit): void;
        walk(): TreeCursor;
        getChangedRanges(other: Tree): Range[];
        getEditedRange(other: Tree): Range;
        printDotGraph(fd?: number): void;
    }

    export interface Language {
        version: number;
        fieldCount: number;
        nodeTypeCount: number;
        stateCount: number;

        fieldNameForId(fieldId: number): string | null;
        fieldIdForName(fieldName: string): number | null;
        idForNodeType(type: string, named: boolean): number;
        nodeTypeForId(typeId: number): string | null;
        nodeTypeIsNamed(typeId: number): boolean;
        nodeTypeIsVisible(typeId: number): boolean;
        query(source: string): Query;
    }

    export interface QueryCapture {
        name: string;
        node: SyntaxNode;
    }

    export interface QueryMatch {
        pattern: number;
        captures: QueryCapture[];
    }

    export interface Query {
        captureNames: string[];
        predicates: any[][];

        captures(node: SyntaxNode, startPosition?: Point, endPosition?: Point): QueryCapture[];
        matches(node: SyntaxNode, startPosition?: Point, endPosition?: Point): QueryMatch[];
    }

    export default class Parser {
        static Language: Language;

        setLanguage(language: Language): void;
        getLanguage(): Language | null;
        parse(input: string | ((index: number, position?: Point) => string | null), previousTree?: Tree, options?: { bufferSize?: number, includedRanges?: Range[] }): Tree;
        reset(): void;
        setTimeoutMicros(timeout: number): void;
        getTimeoutMicros(): number;
        setLogger(logger: ((message: string, params: object, type: 'parse' | 'lex') => void) | null): void;
        getLogger(): ((message: string, params: object, type: 'parse' | 'lex') => void) | null;
        printDotGraphs(enabled: boolean): void;
    }

    export { Parser };
}

declare module 'tree-sitter-typescript' {
    import Parser from 'tree-sitter';
    const typescript: Parser.Language;
    const tsx: Parser.Language;
    export { typescript, tsx };
}

declare module 'tree-sitter-javascript' {
    import Parser from 'tree-sitter';
    const language: Parser.Language;
    export = language;
}

declare module 'tree-sitter-python' {
    import Parser from 'tree-sitter';
    const language: Parser.Language;
    export = language;
}

declare module 'tree-sitter-c-sharp' {
    import Parser from 'tree-sitter';
    const language: Parser.Language;
    export = language;
}

declare module 'tree-sitter-rust' {
    import Parser from 'tree-sitter';
    const language: Parser.Language;
    export = language;
}

declare module 'tree-sitter-go' {
    import Parser from 'tree-sitter';
    const language: Parser.Language;
    export = language;
}
