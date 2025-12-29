import type * as vscode from 'vscode';

export interface Dependency {
    file_path: string;
    line: number;
}

export interface Diagnostic {
    /** Error, Warning, Information, or Hint */
    severity: string;
    message: string;
    line: number;
    column: number;
    /** Optional error code from the language server */
    code?: string;
}

export interface SelectionPayload {
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

export interface BatchFile {
    path: string;
    workspace: string;
    git_root: string | null;
    project_root: string;
    line_start: number | null;
    line_end: number | null;
}

export interface BatchPayload {
    files: BatchFile[];
}

export interface KycoHttpConfig {
    port: number;
    token?: string;
    mtime?: number;
}

// Git Extension API types (vscode.git is always available)
export interface GitExtension {
    getAPI(version: number): GitAPI;
}

export interface GitAPI {
    repositories: GitRepository[];
}

export interface GitRepository {
    rootUri: vscode.Uri;
}

// Constants
export const MAX_DEPENDENCIES = 30;
export const LSP_RETRY_DELAY_MS = 500;
export const LSP_MAX_RETRIES = 3;
export const KYCO_AUTH_HEADER = 'X-KYCO-Token';
export const KYCO_DEFAULT_PORT = 9876;
