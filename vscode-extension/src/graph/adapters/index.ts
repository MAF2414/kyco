import { LanguageAdapter } from '../LanguageAdapter';
import { TypeScriptAdapter } from './typescript';
import { PythonAdapter } from './python';
import { CSharpAdapter } from './csharp';
import { RustAdapter } from './rust';
import { GoAdapter } from './go';

/**
 * Registry of all available language adapters
 */
const adapters: LanguageAdapter[] = [
    new TypeScriptAdapter(),
    new PythonAdapter(),
    new CSharpAdapter(),
    new RustAdapter(),
    new GoAdapter(),
];

/**
 * Map of file extensions to adapters for fast lookup
 */
const extensionMap = new Map<string, LanguageAdapter>();

for (const adapter of adapters) {
    for (const ext of adapter.fileExtensions) {
        extensionMap.set(ext.toLowerCase(), adapter);
    }
}

/**
 * Get the appropriate language adapter for a file
 * @param filePath Path to the file or just the extension
 * @returns The language adapter or null if not supported
 */
export function getAdapterForFile(filePath: string): LanguageAdapter | null {
    const ext = filePath.includes('.')
        ? '.' + filePath.split('.').pop()?.toLowerCase()
        : filePath.toLowerCase();

    return extensionMap.get(ext) || null;
}

/**
 * Get all registered adapters
 */
export function getAllAdapters(): LanguageAdapter[] {
    return [...adapters];
}

/**
 * Get adapter by language ID
 */
export function getAdapterById(languageId: string): LanguageAdapter | null {
    return adapters.find(a => a.languageId === languageId) || null;
}

/**
 * Get all supported file extensions
 */
export function getSupportedExtensions(): string[] {
    return Array.from(extensionMap.keys());
}

/**
 * Check if a file extension is supported
 */
export function isExtensionSupported(ext: string): boolean {
    const normalizedExt = ext.startsWith('.') ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
    return extensionMap.has(normalizedExt);
}

export { TypeScriptAdapter, PythonAdapter, CSharpAdapter, RustAdapter, GoAdapter };
