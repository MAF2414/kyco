/**
 * DiffCache - Caching layer for diff analysis
 * Provides LRU cache for baseline content and file diff results
 */

import * as crypto from 'crypto';
import type { NodeDiff, FileDiffCacheEntry, DiffBaseline } from './types';

/**
 * Simple LRU Cache implementation
 */
export class LRUCache<K, V> {
    private cache: Map<K, V> = new Map();
    private readonly maxSize: number;

    constructor(maxSize: number) {
        this.maxSize = maxSize;
    }

    get(key: K): V | undefined {
        const value = this.cache.get(key);
        if (value !== undefined) {
            // Move to end (most recently used)
            this.cache.delete(key);
            this.cache.set(key, value);
        }
        return value;
    }

    set(key: K, value: V): void {
        // Delete existing to update position
        this.cache.delete(key);

        // Evict oldest if at capacity
        if (this.cache.size >= this.maxSize) {
            const firstKey = this.cache.keys().next().value;
            if (firstKey !== undefined) {
                this.cache.delete(firstKey);
            }
        }

        this.cache.set(key, value);
    }

    has(key: K): boolean {
        return this.cache.has(key);
    }

    delete(key: K): boolean {
        return this.cache.delete(key);
    }

    clear(): void {
        this.cache.clear();
    }

    get size(): number {
        return this.cache.size;
    }

    /**
     * Delete all entries matching a predicate
     */
    deleteMatching(predicate: (key: K, value: V) => boolean): number {
        let deleted = 0;
        const toDelete: K[] = [];

        for (const [key, value] of this.cache.entries()) {
            if (predicate(key, value)) {
                toDelete.push(key);
            }
        }

        for (const key of toDelete) {
            this.cache.delete(key);
            deleted++;
        }

        return deleted;
    }
}

/**
 * Cache for diff analysis results
 */
export class DiffCache {
    // Cache for file diff results (keyed by filePath)
    private fileCache: Map<string, FileDiffCacheEntry> = new Map();

    // LRU cache for baseline file content (keyed by "baseline:filePath")
    private baselineContentCache: LRUCache<string, string>;

    // Current baseline hash for invalidation
    private currentBaselineHash: string = '';

    constructor(baselineCacheSize: number = 100) {
        this.baselineContentCache = new LRUCache(baselineCacheSize);
    }

    /**
     * Get cached diff for a file
     */
    getFileDiff(filePath: string, currentHash: string): NodeDiff[] | null {
        const entry = this.fileCache.get(filePath);
        if (!entry) return null;

        // Check if cache is still valid
        if (entry.baselineHash !== this.currentBaselineHash) {
            // Baseline changed, cache invalid
            this.fileCache.delete(filePath);
            return null;
        }

        if (entry.currentHash !== currentHash) {
            // File content changed, cache invalid
            this.fileCache.delete(filePath);
            return null;
        }

        return entry.diffs;
    }

    /**
     * Cache diff results for a file
     */
    setFileDiff(filePath: string, currentHash: string, diffs: NodeDiff[]): void {
        this.fileCache.set(filePath, {
            baselineHash: this.currentBaselineHash,
            currentHash,
            diffs,
            timestamp: new Date(),
        });
    }

    /**
     * Invalidate cache for a specific file
     */
    invalidateFile(filePath: string): void {
        this.fileCache.delete(filePath);
    }

    /**
     * Get cached baseline content
     */
    getBaselineContent(filePath: string): string | undefined {
        const key = `${this.currentBaselineHash}:${filePath}`;
        return this.baselineContentCache.get(key);
    }

    /**
     * Cache baseline content
     */
    setBaselineContent(filePath: string, content: string): void {
        const key = `${this.currentBaselineHash}:${filePath}`;
        this.baselineContentCache.set(key, content);
    }

    /**
     * Update baseline and invalidate related caches
     */
    setBaseline(baseline: DiffBaseline): void {
        const newHash = computeBaselineHash(baseline);

        if (newHash !== this.currentBaselineHash) {
            // Baseline changed, invalidate all caches
            this.currentBaselineHash = newHash;
            this.fileCache.clear();
            // Baseline content cache uses baseline hash in key,
            // so old entries will be evicted naturally by LRU
        }
    }

    /**
     * Get current baseline hash
     */
    getBaselineHash(): string {
        return this.currentBaselineHash;
    }

    /**
     * Clear all caches
     */
    clear(): void {
        this.fileCache.clear();
        this.baselineContentCache.clear();
    }

    /**
     * Get cache statistics
     */
    getStats(): { fileCacheSize: number; baselineContentCacheSize: number } {
        return {
            fileCacheSize: this.fileCache.size,
            baselineContentCacheSize: this.baselineContentCache.size,
        };
    }
}

/**
 * Compute hash of baseline for cache key
 */
export function computeBaselineHash(baseline: DiffBaseline): string {
    const content = `${baseline.type}:${baseline.reference}:${baseline.timestamp.getTime()}`;
    return crypto.createHash('md5').update(content).digest('hex').slice(0, 16);
}

/**
 * Compute hash of file content for cache validation
 */
export function computeContentHash(content: string): string {
    return crypto.createHash('md5').update(content).digest('hex');
}

/**
 * Compute hash of member body for comparison
 * Normalizes whitespace to ignore formatting changes
 */
export function computeBodyHash(bodyText: string): string {
    // Normalize whitespace for comparison
    const normalized = bodyText
        .replace(/\s+/g, ' ')
        .trim();

    return crypto.createHash('md5').update(normalized).digest('hex').slice(0, 16);
}
