import * as vscode from 'vscode';
import * as http from 'http';
import { CodeMapViewProvider } from './webview/CodeMapView';

// Global reference to the code map view provider
let codeMapProvider: CodeMapViewProvider | undefined;

export function activate(context: vscode.ExtensionContext) {
    console.log('Kyco extension is now active');

    // Initialize Code Map View Provider
    codeMapProvider = new CodeMapViewProvider(context.extensionUri, context);

    // Register the webview view provider (for sidebar)
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            CodeMapViewProvider.viewType,
            codeMapProvider
        )
    );

    // Register Commands

    // Original send selection command (existing functionality)
    const sendSelectionCmd = vscode.commands.registerCommand(
        'kyco.sendSelection',
        async () => {
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
        }
    );

    // Open Code Map command
    const openCodeMapCmd = vscode.commands.registerCommand(
        'kyco.openCodeMap',
        async () => {
            if (codeMapProvider) {
                await codeMapProvider.openPanel();
            }
        }
    );

    // Refresh Code Map command
    const refreshCodeMapCmd = vscode.commands.registerCommand(
        'kyco.refreshCodeMap',
        async () => {
            if (codeMapProvider) {
                await codeMapProvider.refresh();
            }
        }
    );

    // Send Map Selection to Agent command
    const sendSelectionToAgentCmd = vscode.commands.registerCommand(
        'kyco.sendSelectionToAgent',
        async () => {
            // This command is triggered from the webview via keybinding
            // The actual implementation is handled in the webview/CodeMapView
            vscode.window.showInformationMessage('Kyco: Use the Code Map to select and send items');
        }
    );

    // Register all commands
    context.subscriptions.push(
        sendSelectionCmd,
        openCodeMapCmd,
        refreshCodeMapCmd,
        sendSelectionToAgentCmd
    );

    // Show welcome message on first activation
    const hasShownWelcome = context.globalState.get('kyco.hasShownWelcome');
    if (!hasShownWelcome) {
        vscode.window.showInformationMessage(
            'Kyco Code Map is ready! Use "Kyco: Open Code Map" command or Ctrl+Alt+M to visualize your codebase.',
            'Open Code Map',
            'Dismiss'
        ).then(selection => {
            if (selection === 'Open Code Map') {
                vscode.commands.executeCommand('kyco.openCodeMap');
            }
        });
        context.globalState.update('kyco.hasShownWelcome', true);
    }
}

export function deactivate() {
    if (codeMapProvider) {
        codeMapProvider.dispose();
    }
}
