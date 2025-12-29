import * as vscode from 'vscode';
import * as path from 'path';
import * as cp from 'child_process';

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
export async function searchWithExternalTool(
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
