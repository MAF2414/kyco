/**
 * Manual test: Claude subagents (`agents` option)
 *
 * Prereqs:
 *  - `cd bridge && npm install`
 *  - `export ANTHROPIC_API_KEY=...`
 *
 * Run:
 *  - `npm run manual:subagents`
 *
 * Notes:
 *  - Uses `permissionMode: "bypassPermissions"` to avoid interactive approvals.
 *  - Prints bridge events as NDJSON to stdout.
 */

import { executeClaudeQuery } from '../claude.js';
import { SessionStore } from '../store.js';
import type { ClaudeQueryRequest } from '../types.js';

if (!process.env.ANTHROPIC_API_KEY) {
  console.error('Missing ANTHROPIC_API_KEY. Set it before running this manual test.');
  process.exit(1);
}

const cwd = process.env.KYCO_SUBAGENTS_CWD ?? process.cwd();

const request: ClaudeQueryRequest = {
  cwd,
  permissionMode: 'bypassPermissions',
  agents: {
    'code-reviewer': {
      description: 'Reviews code for bugs and style issues. Use for targeted reviews.',
      prompt: [
        'You are a strict code reviewer.',
        '',
        'Focus on:',
        '- Correctness',
        '- Security pitfalls',
        '- Edge cases',
        '- Readability',
        '',
        'Be concise and actionable.',
      ].join('\n'),
      tools: ['Read', 'Grep', 'Glob'],
      model: 'sonnet',
    },
  },
  prompt: process.env.KYCO_SUBAGENTS_PROMPT ?? [
    'Delegate via the Task tool to the "code-reviewer" subagent.',
    'Ask it to review `src/domain/agent.rs` for potential issues.',
    'Then return a short summary of findings.',
  ].join('\n'),
};

const store = new SessionStore(':memory:');

for await (const event of executeClaudeQuery(request, store)) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

