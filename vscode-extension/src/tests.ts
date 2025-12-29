import * as vscode from 'vscode';
import * as path from 'path';

export async function findRelatedTests(filePath: string, workspace: string): Promise<string[]> {
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
