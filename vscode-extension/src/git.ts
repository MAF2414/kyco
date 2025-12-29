import * as vscode from 'vscode';
import * as path from 'path';
import type { GitExtension } from './types';

/**
 * Get the Git repository root for a file.
 * Returns null if the file is not in a Git repository.
 */
export function getGitRoot(fileUri: vscode.Uri): string | null {
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
export function getProjectRoot(fileUri: vscode.Uri): string {
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
