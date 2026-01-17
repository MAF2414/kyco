/**
 * BugBounty scope/tool-policy enforcement (tool-layer).
 *
 * The Rust side injects the active BugBounty project's scope/policy into the
 * Claude request env as JSON strings:
 * - KYCO_BUGBOUNTY_PROJECT_ID
 * - KYCO_BUGBOUNTY_PROJECT_ROOT (absolute)
 * - KYCO_BUGBOUNTY_SCOPE_JSON
 * - KYCO_BUGBOUNTY_TOOL_POLICY_JSON
 */

import path from 'node:path';

export interface ProjectScope {
  in_scope?: string[];
  out_of_scope?: string[];
  rate_limit?: number; // requests per second
  notes?: string;
}

export interface ToolPolicy {
  allowed_commands?: string[];
  blocked_commands?: string[];
  network_wrapper?: string;
  protected_paths?: string[];
}

export interface BugBountyPolicy {
  enabled: boolean;
  projectId?: string;
  projectRoot?: string; // absolute path
  scope?: ProjectScope;
  toolPolicy?: ToolPolicy;
}

export function parseBugbountyPolicy(env?: Record<string, string>): BugBountyPolicy | null {
  if (!env) return null;

  const projectId = env.KYCO_BUGBOUNTY_PROJECT_ID?.trim();
  const projectRoot = env.KYCO_BUGBOUNTY_PROJECT_ROOT?.trim();
  const enabled = env.KYCO_BUGBOUNTY_ENFORCE === '1' || !!projectId;

  if (!enabled) return null;

  let scope: ProjectScope | undefined;
  const scopeRaw = env.KYCO_BUGBOUNTY_SCOPE_JSON;
  if (scopeRaw) {
    try {
      scope = JSON.parse(scopeRaw) as ProjectScope;
    } catch {
      // ignore
    }
  }

  let toolPolicy: ToolPolicy | undefined;
  const policyRaw = env.KYCO_BUGBOUNTY_TOOL_POLICY_JSON;
  if (policyRaw) {
    try {
      toolPolicy = JSON.parse(policyRaw) as ToolPolicy;
    } catch {
      // ignore
    }
  }

  return {
    enabled: true,
    projectId: projectId || undefined,
    projectRoot: projectRoot || undefined,
    scope,
    toolPolicy,
  };
}

export type ToolUseDecision =
  | { allow: true; delayMs?: number }
  | { allow: false; reason: string; systemMessage?: string };

const SESSION_DOMAIN_LAST_REQUEST_AT = new Map<string, Map<string, number>>();

export function redactSensitiveText(text: string): string {
  if (!text) return text;

  let out = text;
  out = out.replace(
    /(Authorization:\s*Bearer\s+)[A-Za-z0-9._-]{10,}/gi,
    '$1[REDACTED]',
  );
  out = out.replace(/(Cookie:\s*)([^\n\r]*)/gi, '$1[REDACTED]');
  out = out.replace(/(Set-Cookie:\s*)([^\n\r]*)/gi, '$1[REDACTED]');
  out = out.replace(/(X-API-Key:\s*)([^\n\r]*)/gi, '$1[REDACTED]');
  out = out.replace(/(\bapi[_-]?key\b\s*[:=]\s*)([^\s'"]{8,})/gi, '$1[REDACTED]');
  out = out.replace(/(\btoken\b\s*[:=]\s*)([^\s'"]{8,})/gi, '$1[REDACTED]');
  out = out.replace(/(\bpassword\b\s*[:=]\s*)([^\s'"]{4,})/gi, '$1[REDACTED]');
  return out;
}

export function enforceToolUse(
  sessionId: string,
  toolName: string,
  toolInput: Record<string, unknown>,
  policy: BugBountyPolicy | null,
): ToolUseDecision {
  if (!policy?.enabled) return { allow: true };

  // 1) Protected paths (Read/Write/Edit)
  if (toolName === 'Read' || toolName === 'Write' || toolName === 'Edit') {
    const filePath = String((toolInput as { file_path?: unknown }).file_path ?? '').trim();
    if (filePath) {
      const blocked = isProtectedPath(filePath, policy.projectRoot, policy.toolPolicy?.protected_paths);
      if (blocked) {
        return {
          allow: false,
          reason: `Blocked access to protected path: ${filePath}`,
          systemMessage: `KYCO policy blocked ${toolName} on a protected path. Do not read/write credentials/secrets.`,
        };
      }
    }
  }

  // 2) Network scope + wrapper-only enforcement (Bash)
  if (toolName === 'Bash') {
    const command = String((toolInput as { command?: unknown }).command ?? '').trim();
    if (!command) return { allow: true };

    const networkWrapper = policy.toolPolicy?.network_wrapper?.trim();
    const usesNetworkBinary = /\b(curl|wget)\b/i.test(command);
    if (networkWrapper && usesNetworkBinary) {
      const candidates = networkWrapper.startsWith('./')
        ? [networkWrapper, networkWrapper.slice(2)]
        : [networkWrapper];
      const usesWrapper = candidates.some(c => c && command.includes(c));
      if (!usesWrapper) {
      return {
        allow: false,
        reason: `Network wrapper required (${networkWrapper}). Use the wrapper instead of calling curl/wget directly.`,
        systemMessage: `KYCO policy requires using the configured network wrapper (${networkWrapper}) for network requests.`,
      };
      }
    }

    const hostnames = extractHostnamesFromCommand(command);
    if (hostnames.length > 0 && policy.scope) {
      for (const host of hostnames) {
        if (!isHostnameInScope(host, policy.scope)) {
          return {
            allow: false,
            reason: `Out-of-scope network target: ${host}`,
            systemMessage: `KYCO scope policy blocked a network request to an out-of-scope target (${host}).`,
          };
        }
      }
    }

    const rps = policy.scope?.rate_limit;
    if (rps && rps > 0 && hostnames.length > 0) {
      const delayMs = calculateRateLimitDelayMs(sessionId, hostnames, rps);
      if (delayMs > 0) {
        return { allow: true, delayMs };
      }
    }
  }

  return { allow: true };
}

function isProtectedPath(
  filePath: string,
  projectRoot?: string,
  protectedPaths?: string[],
): boolean {
  if (!projectRoot || !protectedPaths?.length) return false;

  let resolvedFile: string;
  try {
    resolvedFile = path.resolve(filePath);
  } catch {
    return false;
  }

  for (const rel of protectedPaths) {
    const trimmed = rel.trim();
    if (!trimmed) continue;
    const protectedAbs = path.resolve(projectRoot, trimmed);
    if (resolvedFile === protectedAbs) return true;
    const prefix = protectedAbs.endsWith(path.sep) ? protectedAbs : `${protectedAbs}${path.sep}`;
    if (resolvedFile.startsWith(prefix)) return true;
  }

  return false;
}

function extractHostnamesFromCommand(command: string): string[] {
  const urls = command.match(/https?:\/\/[^\s'"]+/gi) ?? [];
  const hostnames = new Set<string>();
  for (const rawUrl of urls) {
    try {
      const u = new URL(rawUrl);
      if (u.hostname) hostnames.add(u.hostname.toLowerCase());
    } catch {
      // ignore
    }
  }
  return Array.from(hostnames);
}

function normalizeScopePattern(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  // If this is a URL, extract hostname.
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    try {
      const u = new URL(trimmed);
      return u.hostname.toLowerCase();
    } catch {
      return null;
    }
  }

  // Strip path fragments and port from patterns like "example.com/path" or "example.com:443".
  const withoutPath = trimmed.split(/[\/\s]/)[0];
  const withoutPort = withoutPath.split(':')[0];
  return withoutPort.toLowerCase();
}

function hostnameMatchesPattern(hostname: string, pattern: string): boolean {
  const p = normalizeScopePattern(pattern);
  if (!p) return false;

  const h = hostname.toLowerCase();

  if (p.includes('*')) {
    // Very small wildcard support: "*" matches any sequence (domain-only).
    const escaped = p.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*');
    const re = new RegExp(`^${escaped}$`, 'i');
    if (re.test(h)) return true;

    // Special-case: "*.example.com" should also match "example.com"
    if (p.startsWith('*.')) {
      const base = p.slice(2);
      if (h === base) return true;
    }
    return false;
  }

  if (h === p) return true;
  return h.endsWith(`.${p}`);
}

function isHostnameInScope(hostname: string, scope: ProjectScope): boolean {
  const inScope = scope.in_scope ?? [];
  const outScope = scope.out_of_scope ?? [];

  for (const pat of outScope) {
    if (hostnameMatchesPattern(hostname, pat)) return false;
  }

  if (inScope.length === 0) {
    // No explicit allowlist => best-effort allow (still respects out_of_scope).
    return true;
  }

  for (const pat of inScope) {
    if (hostnameMatchesPattern(hostname, pat)) return true;
  }

  return false;
}

function calculateRateLimitDelayMs(sessionId: string, hostnames: string[], rps: number): number {
  const minIntervalMs = Math.floor(1000 / rps);
  if (minIntervalMs <= 0) return 0;

  const now = Date.now();
  let maxDelay = 0;

  let domainMap = SESSION_DOMAIN_LAST_REQUEST_AT.get(sessionId);
  if (!domainMap) {
    domainMap = new Map<string, number>();
    SESSION_DOMAIN_LAST_REQUEST_AT.set(sessionId, domainMap);
  }

  for (const host of hostnames) {
    const last = domainMap.get(host) ?? 0;
    const elapsed = now - last;
    if (elapsed < minIntervalMs) {
      maxDelay = Math.max(maxDelay, minIntervalMs - elapsed);
    }
  }

  // Reserve the slot: treat the next request time as `now + maxDelay` so follow-up tool uses
  // don't accidentally burst while we are sleeping in the hook.
  const scheduled = now + maxDelay;
  for (const host of hostnames) {
    domainMap.set(host, scheduled);
  }

  return maxDelay;
}
