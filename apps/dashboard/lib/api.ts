import type {
  CommandResult,
  HealthStatus,
  Portfolio,
  Risk,
  SignalEvent,
  SystemStatus,
} from './types';

/** Resolve API base URL.
 *
 * - `NEXT_PUBLIC_API_URL` set → direct calls (local or external API)
 * - unset in browser → same-origin `/api` proxy via Next.js rewrites (Vercel)
 * - unset on server → `CONTROL_API_URL` fallback
 */
function apiBase(): string {
  if (process.env.NEXT_PUBLIC_API_URL) {
    return process.env.NEXT_PUBLIC_API_URL.replace(/\/$/, '');
  }
  if (typeof window !== 'undefined') {
    return '';
  }
  return (process.env.CONTROL_API_URL ?? 'http://localhost:3001').replace(/\/$/, '');
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${apiBase()}${path}`, { cache: 'no-store' });
  if (!res.ok) throw new Error(`GET ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${apiBase()}${path}`, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(`POST ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

async function patch<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${apiBase()}${path}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`PATCH ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

// ─── API surface ─────────────────────────────────────────────────────────────

export const api = {
  health: ()                        => get<HealthStatus>('/api/health'),
  status: ()                        => get<SystemStatus>('/api/system/status'),
  portfolio: ()                     => get<Portfolio>('/api/portfolio'),
  risk: ()                          => get<Risk>('/api/risk'),
  config: ()                        => get<Record<string, unknown>>('/api/config'),

  signals: (opts?: { limit?: number; signal_type?: string }) => {
    const qs = new URLSearchParams();
    if (opts?.limit)       qs.set('limit', String(opts.limit));
    if (opts?.signal_type) qs.set('signal_type', opts.signal_type);
    const q = qs.toString();
    return get<SignalEvent[]>(`/api/signals${q ? '?' + q : ''}`);
  },

  startSystem:    ()             => post<CommandResult>('/api/system/start'),
  stopSystem:     ()             => post<CommandResult>('/api/system/stop'),
  resetPortfolio: ()             => post<CommandResult>('/api/system/reset-portfolio'),
  patchConfig:    (p: unknown)   => patch<CommandResult>('/api/config', p),
};
