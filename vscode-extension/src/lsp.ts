import * as vscode from 'vscode';
import type { Dependency, Diagnostic } from './types';
import { MAX_DEPENDENCIES, LSP_RETRY_DELAY_MS, LSP_MAX_RETRIES } from './types';
import { getOutputChannel, sleep } from './utils';

/**
 * Fetches references with retry logic for Language Servers that may not be ready yet.
 * This is especially useful for rust-analyzer which needs time to index.
 */
async function fetchReferencesWithRetry(
    document: vscode.TextDocument,
    position: vscode.Position,
    log: vscode.OutputChannel
): Promise<vscode.Location[]> {
    for (let attempt = 1; attempt <= LSP_MAX_RETRIES; attempt++) {
        const references = await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeReferenceProvider',
            document.uri,
            position
        );

        if (references && references.length > 0) {
            return references;
        }

        // Only retry if we got empty results (LSP might not be ready)
        if (attempt < LSP_MAX_RETRIES) {
            log.appendLine(`[KYCo] No references found (attempt ${attempt}/${LSP_MAX_RETRIES}), retrying in ${LSP_RETRY_DELAY_MS}ms...`);
            await sleep(LSP_RETRY_DELAY_MS);
        }
    }

    return [];
}

export async function findDependencies(
    document: vscode.TextDocument,
    selection: vscode.Selection
): Promise<{ dependencies: Dependency[]; totalCount: number; additionalCount: number }> {
    const log = getOutputChannel();

    try {
        const allDependencies: Dependency[] = [];
        const seenLocations = new Set<string>();

        // Handle empty selection: use word at cursor position
        if (selection.isEmpty) {
            log.appendLine(`[KYCo] Empty selection detected at line ${selection.active.line + 1}, char ${selection.active.character}`);

            const wordRange = document.getWordRangeAtPosition(selection.active);
            if (!wordRange) {
                log.appendLine('[KYCo] No word found at cursor position');
                return { dependencies: [], totalCount: 0, additionalCount: 0 };
            }

            const word = document.getText(wordRange);
            log.appendLine(`[KYCo] Using word at cursor: "${word}"`);

            // Use retry logic for empty selection (single word) - LSP might not be ready
            const references = await fetchReferencesWithRetry(document, selection.active, log);

            log.appendLine(`[KYCo] Found ${references.length} references for "${word}"`);

            if (references.length > 0) {
                for (const ref of references) {
                    // Skip references in the same file
                    if (ref.uri.fsPath === document.uri.fsPath) continue;

                    const locationKey = `${ref.uri.fsPath}:${ref.range.start.line}`;
                    if (seenLocations.has(locationKey)) continue;
                    seenLocations.add(locationKey);

                    allDependencies.push({
                        file_path: ref.uri.fsPath,
                        line: ref.range.start.line + 1  // 1-indexed
                    });
                }
            }
        } else {
            // Handle text selection: iterate through all words in selection
            log.appendLine(`[KYCo] Selection: lines ${selection.start.line + 1}-${selection.end.line + 1}, chars ${selection.start.character}-${selection.end.character}`);

            let wordCount = 0;

            for (let line = selection.start.line; line <= selection.end.line; line++) {
                const lineText = document.lineAt(line).text;

                // Find word boundaries in the line
                const wordPattern = /\b[a-zA-Z_][a-zA-Z0-9_]*\b/g;
                let match;

                while ((match = wordPattern.exec(lineText)) !== null) {
                    const startChar = match.index;
                    const endChar = startChar + match[0].length;

                    // Skip if word is completely outside selection
                    // A word is inside selection if it overlaps with the selection range
                    if (line === selection.start.line && endChar <= selection.start.character) continue;
                    if (line === selection.end.line && startChar >= selection.end.character) continue;

                    wordCount++;
                    const position = new vscode.Position(line, startChar);

                    // Use Language Server to find references
                    const references = await vscode.commands.executeCommand<vscode.Location[]>(
                        'vscode.executeReferenceProvider',
                        document.uri,
                        position
                    );

                    if (references) {
                        for (const ref of references) {
                            // Skip references in the same file
                            if (ref.uri.fsPath === document.uri.fsPath) continue;

                            const locationKey = `${ref.uri.fsPath}:${ref.range.start.line}`;
                            if (seenLocations.has(locationKey)) continue;
                            seenLocations.add(locationKey);

                            allDependencies.push({
                                file_path: ref.uri.fsPath,
                                line: ref.range.start.line + 1  // 1-indexed
                            });
                        }
                    }
                }
            }

            log.appendLine(`[KYCo] Processed ${wordCount} words in selection, found ${allDependencies.length} unique dependencies`);
        }

        const totalCount = allDependencies.length;

        // If more than MAX_DEPENDENCIES, return first 30 and count of additional
        if (totalCount > MAX_DEPENDENCIES) {
            return {
                dependencies: allDependencies.slice(0, MAX_DEPENDENCIES),
                totalCount: totalCount,
                additionalCount: totalCount - MAX_DEPENDENCIES
            };
        }

        return {
            dependencies: allDependencies,
            totalCount: totalCount,
            additionalCount: 0
        };
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        log.appendLine(`[KYCo] Error finding dependencies: ${errorMessage}`);
        console.error('Error finding dependencies:', error);
        return {
            dependencies: [],
            totalCount: 0,
            additionalCount: 0
        };
    }
}

/**
 * Get all diagnostics (errors, warnings, etc.) for a document from the Language Server.
 */
export function getDiagnosticsForDocument(document: vscode.TextDocument): Diagnostic[] {
    const vscodeDiagnostics = vscode.languages.getDiagnostics(document.uri);

    return vscodeDiagnostics.map(diag => {
        let severity: string;
        switch (diag.severity) {
            case vscode.DiagnosticSeverity.Error:
                severity = 'Error';
                break;
            case vscode.DiagnosticSeverity.Warning:
                severity = 'Warning';
                break;
            case vscode.DiagnosticSeverity.Information:
                severity = 'Information';
                break;
            case vscode.DiagnosticSeverity.Hint:
                severity = 'Hint';
                break;
            default:
                severity = 'Unknown';
        }

        return {
            severity,
            message: diag.message,
            line: diag.range.start.line + 1,  // 1-indexed
            column: diag.range.start.character + 1,  // 1-indexed
            code: diag.code ? String(diag.code) : undefined
        };
    });
}
