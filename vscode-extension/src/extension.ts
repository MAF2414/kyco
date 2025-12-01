import * as vscode from 'vscode';
import * as http from 'http';

export function activate(context: vscode.ExtensionContext) {
    const disposable = vscode.commands.registerCommand('kyco.sendSelection', async () => {
        const editor = vscode.window.activeTextEditor;

        if (!editor) {
            vscode.window.showErrorMessage('Kyco: No active editor');
            return;
        }

        const document = editor.document;
        const selection = editor.selection;

        // Get absolute file path
        const filePath = document.uri.fsPath;

        // Get selected text (or empty string if none)
        const selectedText = document.getText(selection);

        // Get line numbers (1-indexed in VS Code API)
        const lineStart = selection.start.line + 1;
        const lineEnd = selection.end.line + 1;

        // Get workspace root path
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
        const workspace = workspaceFolder ? workspaceFolder.uri.fsPath : '';

        // Prepare payload
        const payload = JSON.stringify({
            file_path: filePath,
            selected_text: selectedText,
            line_start: lineStart,
            line_end: lineEnd,
            workspace: workspace
        });

        // Send HTTP POST request
        const options = {
            hostname: 'localhost',
            port: 9876,
            path: '/selection',
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Content-Length': Buffer.byteLength(payload)
            }
        };

        const req = http.request(options, (res) => {
            let responseData = '';

            res.on('data', (chunk) => {
                responseData += chunk;
            });

            res.on('end', () => {
                if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
                    vscode.window.showInformationMessage('Kyco: Selection sent successfully');
                } else {
                    vscode.window.showErrorMessage(`Kyco: Server responded with status ${res.statusCode}`);
                }
            });
        });

        req.on('error', (error) => {
            vscode.window.showErrorMessage(`Kyco: Failed to send selection - ${error.message}`);
        });

        req.write(payload);
        req.end();
    });

    context.subscriptions.push(disposable);
}

export function deactivate() {}
