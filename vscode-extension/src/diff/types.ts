/**
 * Types for the DiffAnalyzer module
 * Handles structural diff analysis between baseline and current code
 */

import type { NodeType } from '../graph/types';

/**
 * Worktree information from git
 */
export interface WorktreeInfo {
    path: string;                    // Absolute path
    name: string;                    // Derived name (directory name)
    branch: string;                  // Current branch
    head: string;                    // HEAD commit SHA
    isMain: boolean;                 // Is main worktree?
    isBare: boolean;                 // Bare worktree?
    locked: boolean;                 // Worktree locked?
    prunable: boolean;               // Can be removed?

    // Agent metadata (if created by agent)
    agent?: {
        id: string;
        taskDescription: string;
        createdAt: Date;
        status: 'active' | 'completed' | 'abandoned';
    };

    // Statistics
    stats: WorktreeStats;
}

export interface WorktreeStats {
    filesModified: number;
    uncommittedChanges: boolean;
    aheadOfMain: number;           // Commits ahead of main branch
    behindMain: number;            // Commits behind main branch
}

/**
 * Agent session configuration
 */
export interface AgentSessionConfig {
    name: string;
    mode: 'inline' | 'worktree' | 'pr';
    taskDescription: string;
    baseBranch?: string;            // For worktree/pr mode
    targetBranch?: string;          // For pr mode
}

/**
 * Agent session information
 */
export interface AgentSession {
    id: string;
    name: string;                    // User-friendly name
    mode: 'inline' | 'worktree' | 'pr';

    // For worktree/pr mode
    worktree?: {
        path: string;
        branch: string;
    };

    // For pr mode
    pullRequest?: {
        number: number;
        url: string;
        targetBranch: string;
        status: 'draft' | 'open' | 'merged' | 'closed';
    };

    createdAt: Date;
    lastActivity: Date;
    taskDescription: string;
}

/**
 * Baseline worktree information
 */
export interface BaselineWorktree {
    path: string;
    name: string;
    branch: string;
    isMain: boolean;
}

/**
 * Baseline metadata
 */
export interface BaselineMetadata {
    commitMessage?: string;
    author?: string;
    snapshotNote?: string;
    fileCount?: number;
    agentId?: string;              // Which agent created this state
}

/**
 * Baseline against which diffs are calculated
 */
export interface DiffBaseline {
    type: 'commit' | 'branch' | 'snapshot' | 'worktree' | 'working-tree';
    reference: string;              // SHA, Branch-Name, Snapshot-ID, Worktree-Path
    timestamp: Date;
    label?: string;                 // User-defined label for snapshots

    // Worktree-specific
    worktree?: BaselineWorktree;

    // Extended metadata
    metadata?: BaselineMetadata;
}

/**
 * Member types within a class/interface
 */
export type MemberType = 'method' | 'property' | 'constructor' | 'getter' | 'setter';

/**
 * Severity levels for changes
 */
export type DiffSeverity = 'none' | 'low' | 'medium' | 'high';

/**
 * Change status for nodes/edges/members
 */
export type ChangeStatus = 'unchanged' | 'modified' | 'added' | 'removed';

/**
 * Detailed changes for a modified member
 */
export interface MemberChanges {
    signatureChanged: boolean;
    beforeSignature?: string;
    afterSignature?: string;
    bodyChanged: boolean;
    linesAdded: number;
    linesRemoved: number;
}

/**
 * Diff for a single member (method, property, etc.)
 */
export interface MemberDiff {
    memberName: string;
    memberType: MemberType;
    changeType: ChangeStatus;
    severity: DiffSeverity;

    // Present when changeType is 'modified'
    changes?: MemberChanges;
}

/**
 * Summary statistics for node changes
 */
export interface NodeDiffSummary {
    membersAdded: number;
    membersRemoved: number;
    membersModified: number;
    linesAdded: number;
    linesRemoved: number;
    signatureChanges: number;
}

/**
 * Dependency changes (imports)
 */
export interface DependencyChanges {
    added: string[];              // New imports
    removed: string[];            // Deleted imports
}

/**
 * Inheritance/implementation changes for classes
 */
export interface InheritanceChanges {
    beforeExtends?: string;
    afterExtends?: string;
    beforeImplements?: string[];
    afterImplements?: string[];
}

/**
 * Diff for a single node (class, function, interface, namespace)
 */
export interface NodeDiff {
    nodeId: string;
    filePath: string;
    nodeType: NodeType;
    nodeName: string;

    // Overall status
    status: ChangeStatus;
    overallSeverity: DiffSeverity;

    // Aggregated metrics
    summary: NodeDiffSummary;

    // Detail: Changes per member
    memberDiffs: MemberDiff[];

    // Dependency changes
    dependencyChanges: DependencyChanges;

    // For classes: inheritance changes
    inheritanceChanges?: InheritanceChanges;
}

/**
 * Diff for an edge (relationship between nodes)
 */
export interface EdgeDiff {
    edgeId: string;
    fromNodeId: string;
    toNodeId: string;
    status: ChangeStatus;

    // For modified edges (e.g., import path changed)
    beforeDetail?: string;
    afterDetail?: string;
}

/**
 * Statistics breakdown by severity
 */
export interface SeverityStats {
    low: number;
    medium: number;
    high: number;
}

/**
 * Overall statistics for the graph diff
 */
export interface GraphDiffStats {
    totalNodesChanged: number;
    totalEdgesChanged: number;
    bySeverity: SeverityStats;
}

/**
 * Complete diff result for the entire graph
 */
export interface GraphDiff {
    baseline: DiffBaseline;
    calculatedAt: Date;

    nodeDiffs: Map<string, NodeDiff>;
    edgeDiffs: Map<string, EdgeDiff>;

    // New nodes that didn't exist in baseline
    addedNodes: string[];
    // Deleted nodes that existed in baseline
    removedNodes: string[];

    // Statistics
    stats: GraphDiffStats;
}

/**
 * Information about a member extracted from AST
 */
export interface MemberInfo {
    name: string;
    type: MemberType;
    signature: string;
    bodyHash: string;
    rawText: string;
    lineStart: number;
    lineEnd: number;
    isExported: boolean;
}

/**
 * Cache entry for file diff results
 */
export interface FileDiffCacheEntry {
    baselineHash: string;
    currentHash: string;
    diffs: NodeDiff[];
    timestamp: Date;
}

/**
 * Event payload for diff changes
 */
export interface DiffChangedEvent {
    nodeIds: string[];
    graphDiff: GraphDiff;
}

/**
 * Configuration for the DiffAnalyzer
 */
export interface DiffAnalyzerConfig {
    // Debounce time for file watcher (ms)
    debounceMs: number;

    // Max files to cache baseline content
    baselineCacheSize: number;

    // Whether to watch for file changes
    enableFileWatcher: boolean;
}

/**
 * Default configuration values
 */
export const DEFAULT_DIFF_CONFIG: DiffAnalyzerConfig = {
    debounceMs: 300,
    baselineCacheSize: 100,
    enableFileWatcher: true,
};

/**
 * File diff when comparing worktrees
 */
export interface WorktreeFileDiff {
    file: string;
    status: 'added' | 'removed' | 'modified';
    inWorktree?: string;            // For added/removed, which worktree has it
}

/**
 * Result of comparing two worktrees
 */
export interface WorktreeDiff {
    worktreeA: string;
    worktreeB: string;
    diffs: WorktreeFileDiff[];
}

/**
 * Event for worktree changes
 */
export interface WorktreeChangedEvent {
    worktree: WorktreeInfo;
    changeType: 'created' | 'updated' | 'removed';
}

/**
 * Agent metadata stored in worktree
 */
export interface AgentWorktreeMetadata {
    id: string;
    name: string;
    taskDescription: string;
    createdAt: string;              // ISO string
    status: 'active' | 'completed' | 'abandoned';
}
