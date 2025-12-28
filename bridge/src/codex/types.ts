/**
 * Local type definitions for Codex module
 */

export type CodexUserInput =
  | { type: 'text'; text: string }
  | { type: 'local_image'; path: string };
