/**
 * Claude Agent SDK Integration
 *
 * Wraps the Claude Agent SDK to provide a streaming interface for KYCO.
 */

export { executeClaudeQuery, interruptClaudeQuery, setClaudePermissionMode } from './query.js';
export { resolveToolApproval, getPendingApprovals } from './approvals.js';
