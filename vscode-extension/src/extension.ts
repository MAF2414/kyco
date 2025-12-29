import * as vscode from 'vscode';
import { handleSendSelection, handleSendBatch, handleSendGrep } from './commands';

export function activate(context: vscode.ExtensionContext) {
    console.log('KYCo extension activated');

    context.subscriptions.push(
        vscode.commands.registerCommand('kyco.sendSelection', handleSendSelection),
        vscode.commands.registerCommand('kyco.sendBatch', (clickedFile, selectedFiles) =>
            handleSendBatch(context, clickedFile, selectedFiles)
        ),
        vscode.commands.registerCommand('kyco.sendGrep', handleSendGrep)
    );
}

export function deactivate() {
    console.log('KYCo extension deactivated');
}
