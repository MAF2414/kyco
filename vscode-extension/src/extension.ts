import * as vscode from 'vscode';
import * as http from 'http';
import * as path from 'path';

interface Dependency {
    file_path: string;
    line: number;
}

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

const MAX_DEPENDENCIES = 30;

export function activate(context: vscode.ExtensionContext) {
    console.log('KYCo extension activated');

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

            sendRequest(payload);
        }
    );

    context.subscriptions.push(sendSelectionCommand);
}

async function findDependencies(
    document: vscode.TextDocument,
    selection: vscode.Selection
): Promise<{ dependencies: Dependency[]; totalCount: number; additionalCount: number }> {
    try {
        // Get all symbol references in the selection range
        const allDependencies: Dependency[] = [];
        const seenLocations = new Set<string>();

        // Iterate through each position in the selection to find references
        for (let line = selection.start.line; line <= selection.end.line; line++) {
            const lineText = document.lineAt(line).text;

            // Find word boundaries in the line
            const wordPattern = /\b[a-zA-Z_][a-zA-Z0-9_]*\b/g;
            let match;

            while ((match = wordPattern.exec(lineText)) !== null) {
                const startChar = match.index;
                const position = new vscode.Position(line, startChar);

                // Skip if position is outside selection
                if (line === selection.start.line && startChar < selection.start.character) continue;
                if (line === selection.end.line && startChar > selection.end.character) continue;

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

function sendRequest(payload: SelectionPayload): void {
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

export function deactivate() {
    console.log('KYCo extension deactivated');
}
