/**
 * Diff module exports
 */

// Types
export type {
    DiffBaseline,
    MemberType,
    DiffSeverity,
    ChangeStatus,
    MemberChanges,
    MemberDiff,
    NodeDiffSummary,
    DependencyChanges,
    InheritanceChanges,
    NodeDiff,
    EdgeDiff,
    SeverityStats,
    GraphDiffStats,
    GraphDiff,
    MemberInfo,
    FileDiffCacheEntry,
    DiffChangedEvent,
    DiffAnalyzerConfig,
    // Worktree types
    WorktreeInfo,
    WorktreeStats,
    WorktreeDiff,
    WorktreeFileDiff,
    WorktreeChangedEvent,
    AgentWorktreeMetadata,
    // Agent session types
    AgentSession,
    AgentSessionConfig,
    BaselineWorktree,
    BaselineMetadata,
} from './types';

export { DEFAULT_DIFF_CONFIG } from './types';

// DiffAnalyzer
export { DiffAnalyzer, createDiffAnalyzer } from './DiffAnalyzer';

// BaselineResolver
export {
    BaselineResolver,
    GitBaselineResolver,
    SnapshotBaselineResolver,
    type IBaselineResolver,
} from './BaselineResolver';

// ASTDiffEngine
export { ASTDiffEngine, type IASTDiffEngine } from './ASTDiffEngine';

// DiffCache
export {
    DiffCache,
    LRUCache,
    computeBaselineHash,
    computeContentHash,
    computeBodyHash,
} from './DiffCache';

// DiffFileWatcher
export {
    DiffFileWatcher,
    createDiffFileWatcher,
    type DiffFileWatcherConfig,
} from './DiffFileWatcher';

// Severity utilities
export {
    aggregateSeverity,
    calculateNodeSeverity,
    classifyAddedMember,
    classifyRemovedMember,
    classifyModifiedMember,
    isImportReorderingOnly,
    calculateLineDiff,
    summarizeNodeChanges,
} from './severity';

// WorktreeManager
export {
    WorktreeManager,
    createWorktreeManager,
    type IWorktreeManager,
} from './WorktreeManager';

// WorktreeBaselineResolver
export {
    WorktreeBaselineResolver,
    createWorktreeBaselineResolver,
} from './WorktreeBaselineResolver';

// AgentSessionManager
export {
    AgentSessionManager,
    createAgentSessionManager,
    type IAgentSessionManager,
    type IGitHubService,
    type PullRequest,
} from './AgentSessionManager';
