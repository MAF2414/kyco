import * as vscode from 'vscode';
import * as http from 'http';
import * as path from 'path';

interface Dependency {
    file_path: string;
    line: number;
}

// Output channel for debugging - created lazily
let outputChannel: vscode.OutputChannel | undefined;

interface SelectionPayload {
    file_path: string;
    selected_text: string;
    line_start: number;
    line_end: number;
    workspace: string;
    dependencies: Dependency[];
    dependency_count: number;
    additional_dependency_count: number;
    related_tests: string[];
}

interface BatchFile {
    path: string;
    workspace: string;
    line_start: number | null;
    line_end: number | null;
}

interface BatchPayload {
    files: BatchFile[];
}

const MAX_DEPENDENCIES = 30;
const LSP_RETRY_DELAY_MS = 500;
const LSP_MAX_RETRIES = 3;

export function activate(context: vscode.ExtensionContext) {
    console.log('KYCo extension activated');

    // Command 1: Send single selection
    const sendSelectionCommand = vscode.commands.registerCommand(
        'kyco.sendSelection',
        async () => {
            const editor = vscode.window.activeTextEditor;

            if (!editor) {
                vscode.window.showErrorMessage('KYCo: No active editor');
                return;
            }

            const document = editor.document;
            const selection = editor.selection;

            // Get file path
            const filePath = document.uri.fsPath;

            // Get selected text
            const selectedText = document.getText(selection);

            // Get line numbers (1-indexed to match JetBrains)
            const lineStart = selection.start.line + 1;
            const lineEnd = selection.end.line + 1;

            // Get workspace path
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
            const workspace = workspaceFolder?.uri.fsPath ?? '';

            // Get dependencies via Language Server
            const { dependencies, totalCount, additionalCount } = await findDependencies(document, selection);

            // Find related test files
            const relatedTests = await findRelatedTests(filePath, workspace);

            const payload: SelectionPayload = {
                file_path: filePath,
                selected_text: selectedText,
                line_start: lineStart,
                line_end: lineEnd,
                workspace: workspace,
                dependencies: dependencies,
                dependency_count: totalCount,
                additional_dependency_count: additionalCount,
                related_tests: relatedTests
            };

            sendSelectionRequest(payload);
        }
    );

    // Command 2: Send batch (multiple files selected in explorer)
    const sendBatchCommand = vscode.commands.registerCommand(
        'kyco.sendBatch',
        async (clickedFile: vscode.Uri | undefined, selectedFiles: vscode.Uri[] | undefined) => {
            // Get files from explorer selection
            let files: vscode.Uri[] = [];

            if (selectedFiles && selectedFiles.length > 0) {
                // Called from explorer context menu with multiple selection
                files = selectedFiles;
            } else if (clickedFile) {
                // Called from explorer context menu with single file
                files = [clickedFile];
            } else {
                vscode.window.showErrorMessage('KYCo: No files selected. Right-click on files in the explorer.');
                return;
            }

            // Filter out directories, keep only files
            const fileStats = await Promise.all(
                files.map(async (uri) => {
                    try {
                        const stat = await vscode.workspace.fs.stat(uri);
                        return { uri, isFile: stat.type === vscode.FileType.File };
                    } catch {
                        return { uri, isFile: false };
                    }
                })
            );
            const validFiles = fileStats.filter(f => f.isFile).map(f => f.uri);

            if (validFiles.length === 0) {
                vscode.window.showErrorMessage('KYCo: No valid files selected');
                return;
            }

            // Get workspace
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            const workspace = workspaceFolder?.uri.fsPath ?? '';

            const batchFiles: BatchFile[] = validFiles.map(uri => ({
                path: uri.fsPath,
                workspace: workspace,
                line_start: null,
                line_end: null
            }));

            const payload: BatchPayload = { files: batchFiles };
            sendBatchRequest(payload, validFiles.length);
        }
    );

    // Command 3: Grep and send matching files as batch
    const sendGrepCommand = vscode.commands.registerCommand(
        'kyco.sendGrep',
        async () => {
            // Ask user for search pattern
            const pattern = await vscode.window.showInputBox({
                prompt: 'Enter search pattern (regex supported)',
                placeHolder: 'e.g., TODO|FIXME or function\\s+\\w+'
            });

            if (!pattern) {
                return; // User cancelled
            }

            // Ask for file glob pattern (optional)
            const globPattern = await vscode.window.showInputBox({
                prompt: 'File pattern (leave empty for all files)',
                placeHolder: 'e.g., **/*.ts or src/**/*.rs',
                value: '**/*'
            });

            if (globPattern === undefined) {
                return; // User cancelled
            }

            const finalGlob = globPattern || '**/*';

            // Show progress while searching
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'KYCo: Searching files...',
                    cancellable: true
                },
                async (progress, token) => {
                    try {
                        // Use VS Code's built-in search
                        const files = await vscode.workspace.findFiles(
                            finalGlob,
                            '**/node_modules/**',
                            1000 // Max files to search
                        );

                        if (token.isCancellationRequested) {
                            return;
                        }

                        // Search for pattern in files
                        const regex = new RegExp(pattern);
                        const matchingFiles: vscode.Uri[] = [];

                        for (const file of files) {
                            if (token.isCancellationRequested) {
                                return;
                            }

                            try {
                                const content = await vscode.workspace.fs.readFile(file);
                                const text = Buffer.from(content).toString('utf8');

                                if (regex.test(text)) {
                                    matchingFiles.push(file);
                                }
                            } catch {
                                // Skip files that can't be read
                            }

                            progress.report({
                                message: `Checked ${matchingFiles.length} matches in ${files.indexOf(file) + 1}/${files.length} files`
                            });
                        }

                        if (matchingFiles.length === 0) {
                            vscode.window.showInformationMessage(`KYCo: No files matching "${pattern}" found`);
                            return;
                        }

                        // Confirm with user
                        const confirm = await vscode.window.showInformationMessage(
                            `Found ${matchingFiles.length} files matching "${pattern}". Send to KYCo?`,
                            'Send',
                            'Cancel'
                        );

                        if (confirm !== 'Send') {
                            return;
                        }

                        // Get workspace
                        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
                        const workspace = workspaceFolder?.uri.fsPath ?? '';

                        const batchFiles: BatchFile[] = matchingFiles.map(uri => ({
                            path: uri.fsPath,
                            workspace: workspace,
                            line_start: null,
                            line_end: null
                        }));

                        const payload: BatchPayload = { files: batchFiles };
                        sendBatchRequest(payload, matchingFiles.length);

                    } catch (error) {
                        const errorMessage = error instanceof Error ? error.message : String(error);
                        vscode.window.showErrorMessage(`KYCo: Search failed - ${errorMessage}`);
                    }
                }
            );
        }
    );

    context.subscriptions.push(sendSelectionCommand);
    context.subscriptions.push(sendBatchCommand);
    context.subscriptions.push(sendGrepCommand);
}

function getOutputChannel(): vscode.OutputChannel {
    if (!outputChannel) {
        outputChannel = vscode.window.createOutputChannel('KYCo');
    }
    return outputChannel;
}

async function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

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

async function findDependencies(
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

async function findRelatedTests(filePath: string, workspace: string): Promise<string[]> {
    if (!workspace) return [];

    try {
        const fileName = path.basename(filePath);
        const fileNameWithoutExt = fileName.replace(/\.[^.]+$/, '');

        // Language-agnostic test file patterns
        // Works for: TypeScript, JavaScript, Python, C#, Java, Go, Rust, etc.
        const testPatterns = [
            // Standard patterns: file.test.ext, file.spec.ext
            `**/${fileNameWithoutExt}.test.*`,
            `**/${fileNameWithoutExt}.spec.*`,
            `**/${fileNameWithoutExt}_test.*`,
            `**/${fileNameWithoutExt}Test.*`,
            `**/${fileNameWithoutExt}Tests.*`,
            `**/${fileNameWithoutExt}Spec.*`,
            // Prefix patterns: test_file.ext, Test_file.ext
            `**/test_${fileNameWithoutExt}.*`,
            `**/Test${fileNameWithoutExt}.*`,
            // Directory patterns: tests/file.ext, test/file.ext, __tests__/file.ext
            `**/tests/${fileNameWithoutExt}.*`,
            `**/test/${fileNameWithoutExt}.*`,
            `**/__tests__/${fileNameWithoutExt}.*`,
        ];

        const relatedTests: string[] = [];
        // Exclude common dependency/build directories
        const excludePattern = '{**/node_modules/**,**/bin/**,**/obj/**,**/target/**,**/.venv/**,**/venv/**,**/__pycache__/**}';

        for (const pattern of testPatterns) {
            const files = await vscode.workspace.findFiles(
                new vscode.RelativePattern(workspace, pattern),
                excludePattern,
                10  // Limit results per pattern
            );

            for (const file of files) {
                if (!relatedTests.includes(file.fsPath)) {
                    relatedTests.push(file.fsPath);
                }
            }
        }

        return relatedTests;
    } catch (error) {
        console.error('Error finding related tests:', error);
        return [];
    }
}

function sendSelectionRequest(payload: SelectionPayload): void {
    const jsonPayload = JSON.stringify(payload);

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: 9876,
        path: '/selection',
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(jsonPayload)
        },
        timeout: 5000
    };

    const req = http.request(options, (res) => {
        if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            vscode.window.showInformationMessage('KYCo: Selection sent successfully');
        } else {
            vscode.window.showErrorMessage(`KYCo: Server responded with status ${res.statusCode}`);
        }
    });

    req.on('error', (err) => {
        vscode.window.showErrorMessage(`KYCo: Failed to send selection - ${err.message}`);
    });

    req.on('timeout', () => {
        req.destroy();
        vscode.window.showErrorMessage('KYCo: Request timed out');
    });

    req.write(jsonPayload);
    req.end();
}

function sendBatchRequest(payload: BatchPayload, fileCount: number): void {
    const jsonPayload = JSON.stringify(payload);

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: 9876,
        path: '/batch',
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(jsonPayload)
        },
        timeout: 5000
    };

    const req = http.request(options, (res) => {
        if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            vscode.window.showInformationMessage(`KYCo: Batch sent (${fileCount} files)`);
        } else {
            vscode.window.showErrorMessage(`KYCo: Server responded with status ${res.statusCode}`);
        }
    });

    req.on('error', (err) => {
        vscode.window.showErrorMessage(`KYCo: Failed to send batch - ${err.message}`);
    });

    req.on('timeout', () => {
        req.destroy();
        vscode.window.showErrorMessage('KYCo: Request timed out');
    });

    req.write(jsonPayload);
    req.end();
}

export function deactivate() {
    console.log('KYCo extension deactivated');
}
