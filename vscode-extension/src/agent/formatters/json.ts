import type { SelectionContext } from '../../graph/types';

/**
 * Serialize SelectionContext to JSON format
 * Uses native JSON.stringify with pretty printing
 */
export function toJSON(context: SelectionContext): string {
    return JSON.stringify({ selection: context }, null, 2);
}
