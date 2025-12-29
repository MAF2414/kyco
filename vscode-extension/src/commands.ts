import * as vscode from 'vscode';
import type { BatchFile, SelectionPayload, BatchPayload } from './types';
import { getGitRoot, getProjectRoot } from './git';
import { normalizeSubtreeGlob } from './utils';
import { findDependencies, getDiagnosticsForDocument } from './lsp';
import { findRelatedTests } from './tests';
import { sendSelectionRequest, sendBatchRequest } from './http';
import { searchWithExternalTool } from './search';
import { getKycoHttpConfig } from './config';

export async function handleSendSelection(): Promise<void> {
    const editor = vscode.window.activeTextEditor;

    if (!editor) {
        vscode.window.showErrorMessage('KYCo: No active editor');
        return;
    }

    const document = editor.document;
    const selection = editor.selection;
    const filePath = document.uri.fsPath;
    const selectedText = document.getText(selection);
    const lineStart = selection.start.line + 1;
    const lineEnd = selection.end.line + 1;
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    const workspace = workspaceFolder?.uri.fsPath ?? '';
    const gitRoot = getGitRoot(document.uri);
    const projectRoot = getProjectRoot(document.uri);

    const { dependencies, totalCount, additionalCount } = await findDependencies(document, selection);
    const relatedTests = await findRelatedTests(filePath, workspace);
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

export async function handleSendBatch(
    context: vscode.ExtensionContext,
    clickedFile: vscode.Uri | undefined,
    selectedFiles: vscode.Uri[] | undefined
): Promise<void> {
    let files: vscode.Uri[] = [];

    if (selectedFiles && selectedFiles.length > 0) {
        files = selectedFiles;
    } else if (clickedFile) {
        files = [clickedFile];
    } else {
        vscode.window.showErrorMessage('KYCo: Nothing selected. Right-click on files/folders in the explorer.');
        return;
    }

    const contextUri = clickedFile ?? files[0];
    const contextWorkspaceFolder =
        (contextUri ? vscode.workspace.getWorkspaceFolder(contextUri) : undefined) ??
        vscode.workspace.workspaceFolders?.[0];
    const workspace = contextWorkspaceFolder?.uri.fsPath ?? '';

    const fileStats = await Promise.all(
        files.map(async (uri) => {
            try {
                const stat = await vscode.workspace.fs.stat(uri);
                return {
                    uri,
                    isFile: stat.type === vscode.FileType.File,
                    isDirectory: stat.type === vscode.FileType.Directory
                };
            } catch {
                return { uri, isFile: false, isDirectory: false };
            }
        })
    );

    const selectedFileUris = fileStats.filter(f => f.isFile).map(f => f.uri);
    const selectedDirUris = fileStats.filter(f => f.isDirectory).map(f => f.uri);

    let expandedDirFiles: vscode.Uri[] = [];
    if (selectedDirUris.length > 0) {
        const defaultGlob = '**/*.{sol,ts,js,tsx,jsx,py}';
        const lastGlob = context.globalState.get<string>('kyco.sendBatch.folderGlob') ?? defaultGlob;

        const rawGlob = await vscode.window.showInputBox({
            prompt: 'Folder file glob (supports **, {}, e.g. **/*.{sol,ts,js,tsx,jsx,py})',
            placeHolder: defaultGlob,
            value: lastGlob
        });

        if (rawGlob === undefined) return;

        const folderGlob = normalizeSubtreeGlob(rawGlob);
        await context.globalState.update('kyco.sendBatch.folderGlob', folderGlob);

        const excludePattern = '{**/node_modules/**,**/.git/**,**/target/**,**/dist/**,**/build/**,**/out/**,**/.idea/**,**/.venv/**,**/venv/**,**/__pycache__/**}';

        expandedDirFiles = await vscode.window.withProgress(
            { location: vscode.ProgressLocation.Notification, title: 'KYCo: Collecting files from folders...', cancellable: true },
            async (_progress, token) => {
                const all: vscode.Uri[] = [];
                for (const dirUri of selectedDirUris) {
                    if (token.isCancellationRequested) break;
                    try {
                        const found = await vscode.workspace.findFiles(
                            new vscode.RelativePattern(dirUri.fsPath, folderGlob),
                            excludePattern, 5000, token
                        );
                        all.push(...found);
                    } catch { /* Ignore individual folder errors */ }
                }
                return all;
            }
        );
    }

    const fileByPath = new Map<string, vscode.Uri>();
    for (const uri of [...selectedFileUris, ...expandedDirFiles]) {
        fileByPath.set(uri.fsPath, uri);
    }
    const allFiles = [...fileByPath.values()];

    if (allFiles.length === 0) {
        vscode.window.showErrorMessage('KYCo: No valid files selected');
        return;
    }

    if (selectedDirUris.length > 0) {
        const confirm = await vscode.window.showInformationMessage(
            `Found ${allFiles.length} files. Send to KYCo?`, 'Send', 'Cancel'
        );
        if (confirm !== 'Send') return;
    }

    const batchFiles: BatchFile[] = allFiles.map(uri => ({
        path: uri.fsPath,
        workspace: workspace,
        git_root: getGitRoot(uri),
        project_root: getProjectRoot(uri),
        line_start: null,
        line_end: null
    }));

    const payload: BatchPayload = { files: batchFiles };
    const kycoHttp = await getKycoHttpConfig(workspace);
    sendBatchRequest(payload, allFiles.length, kycoHttp);
}

export async function handleSendGrep(): Promise<void> {
    const pattern = await vscode.window.showInputBox({
        prompt: 'Enter search pattern (regex supported)',
        placeHolder: 'e.g., TODO|FIXME or function\\s+\\w+'
    });

    if (!pattern) return;

    const globPattern = await vscode.window.showInputBox({
        prompt: 'File pattern (leave empty for all files)',
        placeHolder: 'e.g., *.ts or *.rs (glob for rg, extension for grep)',
        value: ''
    });

    if (globPattern === undefined) return;

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    const workspace = workspaceFolder?.uri.fsPath ?? '';

    if (!workspace) {
        vscode.window.showErrorMessage('KYCo: No workspace folder open');
        return;
    }

    await vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title: 'KYCo: Searching files...', cancellable: true },
        async (progress, token) => {
            try {
                progress.report({ message: 'Running search...' });
                const matchingFiles = await searchWithExternalTool(pattern, globPattern, workspace, token);

                if (token.isCancellationRequested) return;

                if (matchingFiles.length === 0) {
                    vscode.window.showInformationMessage(`KYCo: No files matching "${pattern}" found`);
                    return;
                }

                const confirm = await vscode.window.showInformationMessage(
                    `Found ${matchingFiles.length} files matching "${pattern}". Send to KYCo?`, 'Send', 'Cancel'
                );
                if (confirm !== 'Send') return;

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
