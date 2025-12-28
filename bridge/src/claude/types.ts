/**
 * Internal types for Claude Agent SDK integration
 */

export type ClaudeTextBlock = { type: 'text'; text: string };

export type ClaudeImageBlock = {
  type: 'image';
  source: { type: 'base64'; media_type: string; data: string };
};

export type ClaudeContentBlock = ClaudeTextBlock | ClaudeImageBlock;

export type ClaudeMessageParam = {
  role: 'user';
  content: ClaudeContentBlock[];
};

export type ClaudePromptMessage = {
  type: 'user';
  session_id: string;
  message: ClaudeMessageParam;
  parent_tool_use_id: null;
};

export type EventEmitter = (event: import('../types.js').BridgeEvent) => void;
