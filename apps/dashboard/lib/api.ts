import type {
  CommandResult,
  HealthStatus,
  Portfolio,
  Risk,
  SignalEvent,
  SystemStatus,
} from './types';

const BASE = process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:3001';

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { cache: 'no-store' });
  if (!res.ok) throw new Error(`GET ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(`POST ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

async function patch<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
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
