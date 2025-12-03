/**
 * Severity classification logic for diff analysis
 *
 * Cosmetic (low):
 * - Whitespace, indentation, formatting
 * - Comments added, changed, deleted
 * - Variable renames without behavior change
 * - Import sorting
 *
 * Substantial (medium):
 * - Changed code within existing methods
 * - New or changed control flows (if, loop, switch)
 * - Changed return values with same signature
 * - New private members
 *
 * Structural (high):
 * - Signature changes (parameters, return type)
 * - New or deleted exported symbols
 * - New or deleted dependencies (imports)
 * - Changed inheritance or interface implementation
 * - Deleted methods or classes
 */

import type {
    DiffSeverity,
    MemberDiff,
    NodeDiff,
    MemberInfo,
    DependencyChanges,
    InheritanceChanges,
} from './types';

/**
 * Aggregate severity from multiple member diffs
 */
export function aggregateSeverity(memberDiffs: MemberDiff[]): DiffSeverity {
    if (memberDiffs.length === 0) return 'none';

    const hasHigh = memberDiffs.some(d => d.severity === 'high');
    if (hasHigh) return 'high';

    const hasMedium = memberDiffs.some(d => d.severity === 'medium');
    if (hasMedium) return 'medium';

    const hasLow = memberDiffs.some(d => d.severity === 'low');
    if (hasLow) return 'low';

    return 'none';
}

/**
 * Calculate severity for a node based on all its changes
 */
export function calculateNodeSeverity(
    memberDiffs: MemberDiff[],
    dependencyChanges: DependencyChanges,
    inheritanceChanges?: InheritanceChanges
): DiffSeverity {
    // Check for structural (high) severity conditions first

    // Inheritance/implementation changes are always high
    if (inheritanceChanges) {
        if (inheritanceChanges.beforeExtends !== inheritanceChanges.afterExtends) {
            return 'high';
        }
        const beforeImpl = inheritanceChanges.beforeImplements?.sort().join(',') || '';
        const afterImpl = inheritanceChanges.afterImplements?.sort().join(',') || '';
        if (beforeImpl !== afterImpl) {
            return 'high';
        }
    }

    // New or removed dependencies are high
    if (dependencyChanges.added.length > 0 || dependencyChanges.removed.length > 0) {
        return 'high';
    }

    // Aggregate from member diffs
    return aggregateSeverity(memberDiffs);
}

/**
 * Classify severity for added member
 */
export function classifyAddedMember(member: MemberInfo): DiffSeverity {
    // Exported members are high severity (API change)
    if (member.isExported) {
        return 'high';
    }

    // Private members are medium (internal change)
    return 'medium';
}

/**
 * Classify severity for removed member
 */
export function classifyRemovedMember(member: MemberInfo): DiffSeverity {
    // Exported members are always high severity (breaking change)
    if (member.isExported) {
        return 'high';
    }

    // Private members are medium
    return 'medium';
}

/**
 * Classify severity for modified member
 */
export function classifyModifiedMember(
    before: MemberInfo,
    after: MemberInfo
): DiffSeverity {
    // Signature change is high severity
    if (before.signature !== after.signature) {
        return 'high';
    }

    // Body changed - check if it's just cosmetic
    if (before.bodyHash !== after.bodyHash) {
        // Normalize whitespace to detect cosmetic-only changes
        const normalizedBefore = normalizeForComparison(before.rawText);
        const normalizedAfter = normalizeForComparison(after.rawText);

        if (normalizedBefore === normalizedAfter) {
            // Only whitespace/formatting changed
            return 'low';
        }

        // Check if only comments changed
        const beforeNoComments = removeComments(before.rawText);
        const afterNoComments = removeComments(after.rawText);

        if (normalizeForComparison(beforeNoComments) === normalizeForComparison(afterNoComments)) {
            // Only comments changed
            return 'low';
        }

        // Actual code change
        return 'medium';
    }

    // No effective change
    return 'none';
}

/**
 * Normalize code for comparison (remove whitespace variance)
 */
function normalizeForComparison(code: string): string {
    return code
        .replace(/\s+/g, ' ')        // Collapse whitespace
        .replace(/\s*([{}();,:])\s*/g, '$1')  // Remove space around punctuation
        .trim();
}

/**
 * Remove comments from code for comparison
 */
function removeComments(code: string): string {
    // Remove single-line comments
    let result = code.replace(/\/\/.*$/gm, '');

    // Remove multi-line comments
    result = result.replace(/\/\*[\s\S]*?\*\//g, '');

    // Remove Python-style comments
    result = result.replace(/#.*$/gm, '');

    // Remove doc comments (Python docstrings)
    result = result.replace(/"""[\s\S]*?"""/g, '');
    result = result.replace(/'''[\s\S]*?'''/g, '');

    return result;
}

/**
 * Check if a change is only import reordering
 */
export function isImportReorderingOnly(
    beforeImports: string[],
    afterImports: string[]
): boolean {
    const sortedBefore = [...beforeImports].sort();
    const sortedAfter = [...afterImports].sort();

    return sortedBefore.join(',') === sortedAfter.join(',');
}

/**
 * Calculate line diff statistics
 */
export function calculateLineDiff(beforeText: string, afterText: string): { added: number; removed: number } {
    const beforeLines = beforeText.split('\n');
    const afterLines = afterText.split('\n');

    // Simple line count diff (not a real diff algorithm but good enough for stats)
    const beforeSet = new Set(beforeLines.map(l => l.trim()).filter(l => l));
    const afterSet = new Set(afterLines.map(l => l.trim()).filter(l => l));

    let added = 0;
    let removed = 0;

    for (const line of afterSet) {
        if (!beforeSet.has(line)) {
            added++;
        }
    }

    for (const line of beforeSet) {
        if (!afterSet.has(line)) {
            removed++;
        }
    }

    return { added, removed };
}

/**
 * Summarize changes for a node diff
 */
export function summarizeNodeChanges(
    memberDiffs: MemberDiff[]
): { membersAdded: number; membersRemoved: number; membersModified: number; linesAdded: number; linesRemoved: number; signatureChanges: number } {
    let membersAdded = 0;
    let membersRemoved = 0;
    let membersModified = 0;
    let linesAdded = 0;
    let linesRemoved = 0;
    let signatureChanges = 0;

    for (const diff of memberDiffs) {
        switch (diff.changeType) {
            case 'added':
                membersAdded++;
                break;
            case 'removed':
                membersRemoved++;
                break;
            case 'modified':
                membersModified++;
                if (diff.changes) {
                    linesAdded += diff.changes.linesAdded;
                    linesRemoved += diff.changes.linesRemoved;
                    if (diff.changes.signatureChanged) {
                        signatureChanges++;
                    }
                }
                break;
        }
    }

    return {
        membersAdded,
        membersRemoved,
        membersModified,
        linesAdded,
        linesRemoved,
        signatureChanges,
    };
}
