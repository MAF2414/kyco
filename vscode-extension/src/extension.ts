import * as vscode from 'vscode';
import * as http from 'http';
import * as path from 'path';
import * as cp from 'child_process';

interface Dependency {
    file_path: string;
    line: number;
}

interface Diagnostic {
    /** Error, Warning, Information, or Hint */
    severity: string;
    message: string;
    line: number;
    column: number;
    /** Optional error code from the language server */
    code?: string;
}

// Output channel for debugging - created lazily
let outputChannel: vscode.OutputChannel | undefined;

interface SelectionPayload {
    file_path: string;
    selected_text: string;
    line_start: number;
    line_end: number;
    workspace: string;
    /** Git repository root if file is in a git repo, null otherwise */
    git_root: string | null;
    /** Project root: git_root > workspace_folder > file's parent dir */
    project_root: string;
    dependencies: Dependency[];
    dependency_count: number;
    additional_dependency_count: number;
    related_tests: string[];
    /** Errors and warnings from the language server for this file */
    diagnostics: Diagnostic[];
}

interface BatchFile {
    path: string;
    workspace: string;
    git_root: string | null;
    project_root: string;
    line_start: number | null;
    line_end: number | null;
}

interface BatchPayload {
    files: BatchFile[];
}

interface KycoHttpConfig {
    port: number;
    token?: string;
    mtime?: number;
}

const MAX_DEPENDENCIES = 30;
const LSP_RETRY_DELAY_MS = 500;
const LSP_MAX_RETRIES = 3;
const KYCO_AUTH_HEADER = 'X-KYCO-Token';
const KYCO_DEFAULT_PORT = 9876;

// Git Extension API types (vscode.git is always available)
interface GitExtension {
    getAPI(version: number): GitAPI;
}

interface GitAPI {
    repositories: GitRepository[];
}

interface GitRepository {
    rootUri: vscode.Uri;
}

/**
 * Get the Git repository root for a file.
 * Returns null if the file is not in a Git repository.
 */
function getGitRoot(fileUri: vscode.Uri): string | null {
    try {
        const gitExtension = vscode.extensions.getExtension<GitExtension>('vscode.git');
        if (!gitExtension) {
            return null;
        }

        // Activate the extension if not already active
        if (!gitExtension.isActive) {
            return null; // Don't block on activation
        }

        const gitApi = gitExtension.exports.getAPI(1);
        if (!gitApi || !gitApi.repositories) {
            return null;
        }

        // Find repository containing this file
        const filePath = fileUri.fsPath;
        for (const repo of gitApi.repositories) {
            const repoRoot = repo.rootUri.fsPath;
            if (filePath.startsWith(repoRoot + path.sep) || filePath === repoRoot) {
                return repoRoot;
            }
        }

        return null;
    } catch {
        return null;
    }
}

/**
 * Get the project root for a file with fallback chain:
 * 1. Git repository root (if in a git repo)
 * 2. VSCode workspace folder containing the file
 * 3. Parent directory of the file
 */
function getProjectRoot(fileUri: vscode.Uri): string {
    // Try Git root first
    const gitRoot = getGitRoot(fileUri);
    if (gitRoot) {
        return gitRoot;
    }

    // Fall back to workspace folder
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(fileUri);
    if (workspaceFolder) {
        return workspaceFolder.uri.fsPath;
    }

    // Last resort: parent directory of the file
    return path.dirname(fileUri.fsPath);
}

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

            // Get Git repository root and project root (for correct cwd in agent)
            const gitRoot = getGitRoot(document.uri);
            const projectRoot = getProjectRoot(document.uri);

            // Get dependencies via Language Server
            const { dependencies, totalCount, additionalCount } = await findDependencies(document, selection);

            // Find related test files
            const relatedTests = await findRelatedTests(filePath, workspace);

            // Get diagnostics (errors, warnings) for the current file
            const diagnostics = getDiagnosticsForDocument(document);

            const payload: SelectionPayload = {
                file_path: filePath,
                selected_text: selectedText,
                line_start: lineStart,
                line_end: lineEnd,
                workspace: workspace,
                git_root: gitRoot,
                project_root: projectRoot,
                dependencies: dependencies,
                dependency_count: totalCount,
                additional_dependency_count: additionalCount,
                related_tests: relatedTests,
                diagnostics: diagnostics
            };

            const kycoHttp = await getKycoHttpConfig(workspace);
            sendSelectionRequest(payload, kycoHttp);
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
                git_root: getGitRoot(uri),
                project_root: getProjectRoot(uri),
                line_start: null,
                line_end: null
            }));

            const payload: BatchPayload = { files: batchFiles };
            const kycoHttp = await getKycoHttpConfig(workspace);
            sendBatchRequest(payload, validFiles.length, kycoHttp);
        }
    );

    // Command 3: Grep and send matching files as batch (uses rg/grep for speed)
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
                placeHolder: 'e.g., *.ts or *.rs (glob for rg, extension for grep)',
                value: ''
            });

            if (globPattern === undefined) {
                return; // User cancelled
            }

            // Get workspace
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            const workspace = workspaceFolder?.uri.fsPath ?? '';

            if (!workspace) {
                vscode.window.showErrorMessage('KYCo: No workspace folder open');
                return;
            }

            // Show progress while searching
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'KYCo: Searching files...',
                    cancellable: true
                },
                async (progress, token) => {
                    try {
                        progress.report({ message: 'Running search...' });

                        // Try rg first, then grep, then fallback to JS
                        const matchingFiles = await searchWithExternalTool(pattern, globPattern, workspace, token);

                        if (token.isCancellationRequested) {
                            return;
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

                        const batchFiles: BatchFile[] = matchingFiles.map(filePath => {
                            const uri = vscode.Uri.file(filePath);
                            return {
                                path: filePath,
                                workspace: workspace,
                                git_root: getGitRoot(uri),
                                project_root: getProjectRoot(uri),
                                line_start: null,
                                line_end: null
                            };
                        });

                        const payload: BatchPayload = { files: batchFiles };
                        const kycoHttp = await getKycoHttpConfig(workspace);
                        sendBatchRequest(payload, matchingFiles.length, kycoHttp);

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

/**
 * Get all diagnostics (errors, warnings, etc.) for a document from the Language Server.
 */
function getDiagnosticsForDocument(document: vscode.TextDocument): Diagnostic[] {
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

function sendSelectionRequest(payload: SelectionPayload, kycoHttp?: KycoHttpConfig): void {
    const jsonPayload = JSON.stringify(payload);

    const headers: Record<string, string | number> = {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(jsonPayload)
    };

    // Add auth token if available (required by kyco with http_token enabled)
    if (kycoHttp?.token) {
        headers[KYCO_AUTH_HEADER] = kycoHttp.token;
    }

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: kycoHttp?.port ?? KYCO_DEFAULT_PORT,
        path: '/selection',
        method: 'POST',
        headers,
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

function sendBatchRequest(payload: BatchPayload, fileCount: number, kycoHttp?: KycoHttpConfig): void {
    const jsonPayload = JSON.stringify(payload);

    const headers: Record<string, string | number> = {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(jsonPayload)
    };

    if (kycoHttp?.token) {
        headers[KYCO_AUTH_HEADER] = kycoHttp.token;
    }

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: kycoHttp?.port ?? KYCO_DEFAULT_PORT,
        path: '/batch',
        method: 'POST',
        headers,
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

// ============================================================================
// External search tool (rg/grep) for fast file searching
// ============================================================================

/**
 * Check if a command exists on the system
 */
function commandExists(cmd: string): Promise<boolean> {
    return new Promise((resolve) => {
        const checkCmd = process.platform === 'win32' ? 'where' : 'which';
        cp.exec(`${checkCmd} ${cmd}`, (error) => {
            resolve(!error);
        });
    });
}

/**
 * Search files using external tools (rg > grep) for maximum speed.
 * Returns array of absolute file paths that contain matches.
 */
async function searchWithExternalTool(
    pattern: string,
    globPattern: string,
    cwd: string,
    token: vscode.CancellationToken
): Promise<string[]> {
    // Try ripgrep first (fastest)
    if (await commandExists('rg')) {
        return searchWithRipgrep(pattern, globPattern, cwd, token);
    }

    // Fallback to grep (available on most Unix systems)
    if (process.platform !== 'win32' && await commandExists('grep')) {
        return searchWithGrep(pattern, globPattern, cwd, token);
    }

    // Last resort: JS regex (slowest but always works)
    return searchWithJsRegex(pattern, globPattern, cwd, token);
}

/**
 * Search using ripgrep (rg) - extremely fast, respects .gitignore
 */
function searchWithRipgrep(
    pattern: string,
    globPattern: string,
    cwd: string,
    token: vscode.CancellationToken
): Promise<string[]> {
    return new Promise((resolve, reject) => {
        const args = [
            '--files-with-matches',  // Only output file names
            '--no-heading',
            '--color=never',
            '-e', pattern            // Pattern (supports regex by default)
        ];

        // Add glob filter if provided
        if (globPattern) {
            args.push('--glob', globPattern);
        }

        const proc = cp.spawn('rg', args, { cwd, shell: true });
        const files: string[] = [];
        let stderr = '';

        proc.stdout.on('data', (data: Buffer) => {
            const lines = data.toString().split('\n').filter(line => line.trim());
            for (const line of lines) {
                const filePath = path.isAbsolute(line) ? line : path.join(cwd, line);
                files.push(filePath);
            }
        });

        proc.stderr.on('data', (data: Buffer) => {
            stderr += data.toString();
        });

        proc.on('close', (code) => {
            // rg returns 1 when no matches found, which is not an error
            if (code === 0 || code === 1) {
                resolve(files);
            } else {
                reject(new Error(`rg failed: ${stderr}`));
            }
        });

        proc.on('error', (err) => {
            reject(err);
        });

        // Handle cancellation
        token.onCancellationRequested(() => {
            proc.kill();
            resolve([]);
        });
    });
}

/**
 * Search using grep -r (Unix systems)
 */
function searchWithGrep(
    pattern: string,
    globPattern: string,
    cwd: string,
    token: vscode.CancellationToken
): Promise<string[]> {
    return new Promise((resolve, reject) => {
        // Build grep command
        let cmd = `grep -rlE "${pattern.replace(/"/g, '\\"')}"`;

        // Add include pattern if provided
        if (globPattern) {
            cmd += ` --include="${globPattern}"`;
        }

        // Exclude common directories
        cmd += ' --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=target --exclude-dir=dist --exclude-dir=build';
        cmd += ' .';

        const proc = cp.exec(cmd, { cwd, maxBuffer: 50 * 1024 * 1024 }, (error, stdout, stderr) => {
            if (error && error.code !== 1) {  // grep returns 1 when no matches
                reject(new Error(`grep failed: ${stderr}`));
                return;
            }

            const files = stdout
                .split('\n')
                .filter(line => line.trim())
                .map(line => {
                    // Remove leading ./ if present
                    const cleanPath = line.startsWith('./') ? line.slice(2) : line;
                    return path.join(cwd, cleanPath);
                });

            resolve(files);
        });

        // Handle cancellation
        token.onCancellationRequested(() => {
            proc.kill();
            resolve([]);
        });
    });
}

/**
 * Fallback: Search using JavaScript regex (slowest but always works)
 */
async function searchWithJsRegex(
    pattern: string,
    globPattern: string,
    cwd: string,
    token: vscode.CancellationToken
): Promise<string[]> {
    const finalGlob = globPattern || '**/*';

    const files = await vscode.workspace.findFiles(
        new vscode.RelativePattern(cwd, finalGlob),
        '{**/node_modules/**,**/.git/**,**/target/**,**/dist/**,**/build/**}',
        5000
    );

    if (token.isCancellationRequested) {
        return [];
    }

    const regex = new RegExp(pattern);
    const matchingFiles: string[] = [];

    // Process in parallel batches
    const BATCH_SIZE = 50;
    for (let i = 0; i < files.length; i += BATCH_SIZE) {
        if (token.isCancellationRequested) {
            break;
        }

        const batch = files.slice(i, i + BATCH_SIZE);
        const results = await Promise.all(
            batch.map(async (file) => {
                try {
                    const content = await vscode.workspace.fs.readFile(file);
                    const text = Buffer.from(content).toString('utf8');
                    return regex.test(text) ? file.fsPath : null;
                } catch {
                    return null;
                }
            })
        );

        for (const result of results) {
            if (result) {
                matchingFiles.push(result);
            }
        }
    }

    return matchingFiles;
}

// ============================================================================
// Auth token discovery
// ============================================================================

// In-memory cache keyed by workspace path; refreshed when `.kyco/config.toml` changes.
const httpConfigCache = new Map<string, KycoHttpConfig>();

async function getKycoHttpConfig(workspace: string): Promise<KycoHttpConfig> {
    if (!workspace) {
        return { port: KYCO_DEFAULT_PORT };
    }

    const configUri = vscode.Uri.file(path.join(workspace, '.kyco', 'config.toml'));

    try {
        const stat = await vscode.workspace.fs.stat(configUri);
        const cached = httpConfigCache.get(workspace);
        if (cached && cached.mtime === stat.mtime) {
            return cached;
        }

        const bytes = await vscode.workspace.fs.readFile(configUri);
        const text = Buffer.from(bytes).toString('utf8');
        const token = extractHttpTokenFromToml(text);
        const port = extractHttpPortFromToml(text) ?? KYCO_DEFAULT_PORT;
        const cfg: KycoHttpConfig = { port, token, mtime: stat.mtime };
        httpConfigCache.set(workspace, cfg);
        return cfg;
    } catch {
        // Don't cache failures/misses: users may start `kyco gui` after the first send attempt.
        httpConfigCache.delete(workspace);
        return { port: KYCO_DEFAULT_PORT };
    }
}

function extractHttpTokenFromToml(tomlText: string): string | undefined {
    const lines = tomlText.split(/\r?\n/);
    let inSettingsGui = false;

    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || line.startsWith('#')) {
            continue;
        }

        // Table headers
        if (line.startsWith('[') && line.endsWith(']')) {
            inSettingsGui = line === '[settings.gui]';
            continue;
        }

        if (!inSettingsGui) {
            continue;
        }

        // http_token = "..."
        const mDouble = line.match(/^http_token\\s*=\\s*\"([^\"]*)\"\\s*(?:#.*)?$/);
        if (mDouble) {
            return mDouble[1] || undefined;
        }

        // http_token = '...'
        const mSingle = line.match(/^http_token\\s*=\\s*'([^']*)'\\s*(?:#.*)?$/);
        if (mSingle) {
            return mSingle[1] || undefined;
        }
    }

    return undefined;
}

function extractHttpPortFromToml(tomlText: string): number | undefined {
    const lines = tomlText.split(/\r?\n/);
    let inSettingsGui = false;

    for (const rawLine of lines) {
        const line = rawLine.trim();
        if (!line || line.startsWith('#')) {
            continue;
        }

        if (line.startsWith('[') && line.endsWith(']')) {
            inSettingsGui = line === '[settings.gui]';
            continue;
        }

        if (!inSettingsGui) {
            continue;
        }

        const m = line.match(/^http_port\\s*=\\s*(\\d+)\\s*(?:#.*)?$/);
        if (m) {
            const parsed = Number(m[1]);
            if (Number.isFinite(parsed) && parsed > 0 && parsed < 65536) {
                return parsed;
            }
        }
    }

    return undefined;
}
