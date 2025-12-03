/**
 * AgentSessionManager - Manages agent sessions across different work modes
 * Supports inline, worktree, and PR modes for agent-based code changes
 */

import * as vscode from 'vscode';
import { exec } from 'child_process';
import { promisify } from 'util';
import type {
    AgentSession,
    AgentSessionConfig,
    WorktreeInfo,
    AgentWorktreeMetadata,
} from './types';
import type { IWorktreeManager } from './WorktreeManager';

const execAsync = promisify(exec);

/**
 * Simple event emitter
 */
class EventEmitter<T> {
    private listeners: ((event: T) => void)[] = [];

    on(listener: (event: T) => void): { dispose: () => void } {
        this.listeners.push(listener);
        return {
            dispose: () => {
                const index = this.listeners.indexOf(listener);
                if (index >= 0) this.listeners.splice(index, 1);
            },
        };
    }

    fire(event: T): void {
        for (const listener of this.listeners) {
            listener(event);
        }
    }
}

/**
 * Pull request information
 */
export interface PullRequest {
    number: number;
    url: string;
    html_url: string;
    state: string;
    title: string;
}

/**
 * GitHub service interface (optional, for PR support)
 */
export interface IGitHubService {
    createPullRequest(options: {
        title: string;
        body?: string;
        head: string;
        base: string;
        draft?: boolean;
    }): Promise<PullRequest>;
}

/**
 * Interface for AgentSessionManager
 */
export interface IAgentSessionManager {
    // Sessions
    createSession(config: AgentSessionConfig): Promise<AgentSession>;
    getSession(id: string): AgentSession | null;
    listSessions(): AgentSession[];
    endSession(id: string): Promise<void>;
    updateSessionActivity(id: string): void;

    // Worktree-Integration
    getSessionWorktree(sessionId: string): WorktreeInfo | null;

    // PR-Integration
    createPullRequest(sessionId: string, title: string, body?: string): Promise<PullRequest>;

    // Events
    onSessionCreated: EventEmitter<AgentSession>;
    onSessionEnded: EventEmitter<AgentSession>;
    onSessionUpdated: EventEmitter<AgentSession>;

    // Cleanup
    dispose(): void;
}

/**
 * AgentSessionManager implementation
 */
export class AgentSessionManager implements IAgentSessionManager {
    public onSessionCreated = new EventEmitter<AgentSession>();
    public onSessionEnded = new EventEmitter<AgentSession>();
    public onSessionUpdated = new EventEmitter<AgentSession>();

    private sessions: Map<string, AgentSession> = new Map();
    private worktreeToSession: Map<string, string> = new Map(); // worktreePath -> sessionId

    constructor(
        private worktreeManager: IWorktreeManager,
        private workspaceRoot: string,
        private gitHub?: IGitHubService
    ) {
        // Load existing sessions from worktrees on startup
        this.loadExistingSessions();
    }

    /**
     * Load existing sessions from agent worktrees
     */
    private async loadExistingSessions(): Promise<void> {
        try {
            const worktrees = await this.worktreeManager.listWorktrees();

            for (const worktree of worktrees) {
                if (worktree.agent && !worktree.isMain) {
                    const session: AgentSession = {
                        id: worktree.agent.id,
                        name: worktree.name,
                        mode: 'worktree',
                        worktree: {
                            path: worktree.path,
                            branch: worktree.branch,
                        },
                        createdAt: worktree.agent.createdAt,
                        lastActivity: worktree.agent.createdAt,
                        taskDescription: worktree.agent.taskDescription,
                    };

                    this.sessions.set(session.id, session);
                    this.worktreeToSession.set(worktree.path, session.id);
                }
            }
        } catch (error) {
            console.warn('Failed to load existing sessions:', error);
        }
    }

    /**
     * Generate a unique session ID
     */
    private generateSessionId(): string {
        const timestamp = Date.now().toString(36);
        const random = Math.random().toString(36).substring(2, 8);
        return `agent-${timestamp}-${random}`;
    }

    /**
     * Create a new agent session
     */
    async createSession(config: AgentSessionConfig): Promise<AgentSession> {
        const id = this.generateSessionId();

        const session: AgentSession = {
            id,
            name: config.name,
            mode: config.mode,
            createdAt: new Date(),
            lastActivity: new Date(),
            taskDescription: config.taskDescription,
        };

        // Create worktree if needed
        if (config.mode === 'worktree' || config.mode === 'pr') {
            const worktreeName = `${id}`;
            const worktree = await this.worktreeManager.createWorktree(
                worktreeName,
                config.baseBranch
            );

            session.worktree = {
                path: worktree.path,
                branch: worktree.branch,
            };

            // Save agent metadata in worktree
            const metadata: AgentWorktreeMetadata = {
                id: session.id,
                name: session.name,
                taskDescription: session.taskDescription,
                createdAt: session.createdAt.toISOString(),
                status: 'active',
            };

            await (this.worktreeManager as any).saveAgentMetadata(worktree.path, metadata);
            this.worktreeToSession.set(worktree.path, session.id);
        }

        this.sessions.set(id, session);
        this.onSessionCreated.fire(session);

        return session;
    }

    /**
     * Get a session by ID
     */
    getSession(id: string): AgentSession | null {
        return this.sessions.get(id) || null;
    }

    /**
     * List all sessions
     */
    listSessions(): AgentSession[] {
        return Array.from(this.sessions.values());
    }

    /**
     * Update session activity timestamp
     */
    updateSessionActivity(id: string): void {
        const session = this.sessions.get(id);
        if (session) {
            session.lastActivity = new Date();
            this.onSessionUpdated.fire(session);
        }
    }

    /**
     * Get the worktree for a session
     */
    getSessionWorktree(sessionId: string): WorktreeInfo | null {
        const session = this.sessions.get(sessionId);
        if (!session?.worktree) return null;

        // This would need async access to worktree manager
        // For now, return null - caller should use worktreeManager directly
        return null;
    }

    /**
     * Create a pull request for a PR-mode session
     */
    async createPullRequest(
        sessionId: string,
        title: string,
        body?: string
    ): Promise<PullRequest> {
        const session = this.sessions.get(sessionId);
        if (!session || session.mode !== 'pr') {
            throw new Error('Session not found or not in PR mode');
        }

        if (!session.worktree) {
            throw new Error('Session has no worktree');
        }

        // Push branch to remote
        await execAsync(
            `git push -u origin "${session.worktree.branch}"`,
            { cwd: session.worktree.path }
        );

        // Create PR using gh CLI or GitHub API
        let pr: PullRequest;

        if (this.gitHub) {
            pr = await this.gitHub.createPullRequest({
                title,
                body: body || session.taskDescription,
                head: session.worktree.branch,
                base: 'main',
                draft: true,
            });
        } else {
            // Use gh CLI as fallback
            const prBody = body || session.taskDescription;
            const { stdout } = await execAsync(
                `gh pr create --title "${title}" --body "${prBody.replace(/"/g, '\\"')}" --draft --json number,url,state,title`,
                { cwd: session.worktree.path }
            );

            const prData = JSON.parse(stdout);
            pr = {
                number: prData.number,
                url: prData.url,
                html_url: prData.url,
                state: prData.state,
                title: prData.title,
            };
        }

        // Update session with PR info
        session.pullRequest = {
            number: pr.number,
            url: pr.html_url,
            targetBranch: 'main',
            status: 'draft',
        };

        this.onSessionUpdated.fire(session);

        return pr;
    }

    /**
     * End an agent session
     */
    async endSession(id: string): Promise<void> {
        const session = this.sessions.get(id);
        if (!session) return;

        // Offer to clean up worktree if not merged
        if (session.worktree && !session.pullRequest?.status.includes('merged')) {
            const action = await vscode.window.showQuickPick(
                [
                    { label: 'Remove', description: 'Delete worktree and branch' },
                    { label: 'Keep', description: 'Keep worktree for later' },
                ],
                {
                    placeHolder: `Worktree for "${session.name}" - remove or keep?`,
                }
            );

            if (action?.label === 'Remove') {
                try {
                    await this.worktreeManager.removeWorktree(session.worktree.path, true);
                } catch (error) {
                    vscode.window.showErrorMessage(
                        `Failed to remove worktree: ${error}`
                    );
                }
            }
        }

        // Clean up tracking
        if (session.worktree) {
            this.worktreeToSession.delete(session.worktree.path);
        }

        this.sessions.delete(id);
        this.onSessionEnded.fire(session);
    }

    /**
     * Mark a session as completed
     */
    async markSessionCompleted(id: string): Promise<void> {
        const session = this.sessions.get(id);
        if (!session?.worktree) return;

        // Update metadata in worktree
        const metadata: AgentWorktreeMetadata = {
            id: session.id,
            name: session.name,
            taskDescription: session.taskDescription,
            createdAt: session.createdAt.toISOString(),
            status: 'completed',
        };

        await (this.worktreeManager as any).saveAgentMetadata(session.worktree.path, metadata);
        this.onSessionUpdated.fire(session);
    }

    /**
     * Get session by worktree path
     */
    getSessionByWorktree(worktreePath: string): AgentSession | null {
        const sessionId = this.worktreeToSession.get(worktreePath);
        if (!sessionId) return null;
        return this.sessions.get(sessionId) || null;
    }

    /**
     * Cleanup resources
     */
    dispose(): void {
        this.sessions.clear();
        this.worktreeToSession.clear();
    }
}

/**
 * Factory function for creating AgentSessionManager
 */
export function createAgentSessionManager(
    worktreeManager: IWorktreeManager,
    workspaceRoot: string,
    gitHub?: IGitHubService
): AgentSessionManager {
    return new AgentSessionManager(worktreeManager, workspaceRoot, gitHub);
}
