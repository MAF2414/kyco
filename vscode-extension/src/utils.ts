import * as vscode from 'vscode';

// Output channel for debugging - created lazily
let outputChannel: vscode.OutputChannel | undefined;

export function getOutputChannel(): vscode.OutputChannel {
    if (!outputChannel) {
        outputChannel = vscode.window.createOutputChannel('KYCo');
    }
    return outputChannel;
}

export async function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Normalize a user-provided glob so that simple patterns like `*.sol` match
 * anywhere under the selected folder (e.g., `**` + `/*.sol`).
 */
export function normalizeSubtreeGlob(glob: string): string {
    const trimmed = glob.trim();
    if (!trimmed) {
        return '**/*';
    }

    // If the user already provided a path-aware glob, keep it as-is.
    if (trimmed.includes('/') || trimmed.includes('\\') || trimmed.startsWith('**')) {
        return trimmed;
    }

    return `**/${trimmed}`;
}
