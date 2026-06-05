import type { Direction, SignalEvent, SignalType } from './types';

export type SignalSortKey = 'time' | 'strength' | 'confidence';
export type SignalFilterType = SignalType | 'All';
export type SignalFilterDirection = Direction | 'All';

export interface SignalStats {
  total: number;
  whale: number;
  smartMoney: number;
  momentum: number;
  long: number;
  short: number;
  avgStrength: number;
  avgConfidence: number;
  liveCount: number;
}

export interface OrganizedSignals {
  signals: SignalEvent[];
  liveIds: Set<number>;
  stats: SignalStats;
}

const DISPLAY_CAP = 200;

/** Merge REST history with live WS feed — newest first, deduped by id. */
export function mergeSignals(
  live: SignalEvent[],
  historical: SignalEvent[] | null | undefined,
): OrganizedSignals {
  const liveIds = new Set(live.map((s) => s.signal_id));
  const byId = new Map<number, SignalEvent>();

  for (const s of historical ?? []) {
    byId.set(s.signal_id, s);
  }
  for (const s of live) {
    byId.set(s.signal_id, s);
  }

  const signals = Array.from(byId.values())
    .sort((a, b) => b.timestamp_micros - a.timestamp_micros)
    .slice(0, DISPLAY_CAP);

  const stats = computeStats(signals, liveIds);
  return { signals, liveIds, stats };
}

export function filterSignals(
  signals: SignalEvent[],
  type: SignalFilterType,
  direction: SignalFilterDirection,
): SignalEvent[] {
  return signals.filter((s) => {
    if (type !== 'All' && s.signal_type !== type) return false;
    if (direction !== 'All' && s.direction !== direction) return false;
    return true;
  });
}

export function sortSignals(
  signals: SignalEvent[],
  key: SignalSortKey,
): SignalEvent[] {
  const sorted = [...signals];
  switch (key) {
    case 'strength':
      sorted.sort((a, b) => b.strength - a.strength || b.timestamp_micros - a.timestamp_micros);
      break;
    case 'confidence':
      sorted.sort((a, b) => b.confidence - a.confidence || b.timestamp_micros - a.timestamp_micros);
      break;
    default:
      sorted.sort((a, b) => b.timestamp_micros - a.timestamp_micros);
  }
  return sorted;
}

function computeStats(signals: SignalEvent[], liveIds: Set<number>): SignalStats {
  if (signals.length === 0) {
    return {
      total: 0,
      whale: 0,
      smartMoney: 0,
      momentum: 0,
      long: 0,
      short: 0,
      avgStrength: 0,
      avgConfidence: 0,
      liveCount: 0,
    };
  }

  let whale = 0;
  let smartMoney = 0;
  let momentum = 0;
  let long = 0;
  let short = 0;
  let strengthSum = 0;
  let confidenceSum = 0;
  let liveCount = 0;

  for (const s of signals) {
    if (s.signal_type === 'WhaleFlow') whale += 1;
    if (s.signal_type === 'SmartMoney') smartMoney += 1;
    if (s.signal_type === 'Momentum') momentum += 1;
    if (s.direction === 'Long') long += 1;
    if (s.direction === 'Short') short += 1;
    strengthSum += s.strength;
    confidenceSum += s.confidence;
    if (liveIds.has(s.signal_id)) liveCount += 1;
  }

  return {
    total: signals.length,
    whale,
    smartMoney,
    momentum,
    long,
    short,
    avgStrength: strengthSum / signals.length,
    avgConfidence: confidenceSum / signals.length,
    liveCount,
  };
}

export function formatRelativeTime(timestampMicros: number, nowMs = Date.now()): string {
  const diff = nowMs - timestampMicros / 1000;
  if (diff < 5_000) return 'just now';
  if (diff < 60_000) return `${Math.floor(diff / 1000)}s`;
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  return new Date(timestampMicros / 1000).toLocaleDateString();
}

export function shortPool(address: string, head = 6, tail = 4): string {
  if (address.length <= head + tail + 1) return address;
  return `${address.slice(0, head)}…${address.slice(-tail)}`;
}

export const SIGNAL_TYPE_META: Record<
  SignalType,
  { label: string; color: string; bg: string }
> = {
  WhaleFlow: { label: 'Whale', color: '#4da3ff', bg: '#4da3ff18' },
  SmartMoney: { label: 'Smart $', color: '#a78bfa', bg: '#a78bfa18' },
  Momentum: { label: 'Momentum', color: '#00dfa8', bg: '#00dfa818' },
};

export const DIRECTION_META: Record<Direction, { color: string; bg: string }> = {
  Long: { color: '#22c55e', bg: '#22c55e18' },
  Short: { color: '#ef4444', bg: '#ef444418' },
  Neutral: { color: '#8b95a8', bg: '#8b95a818' },
};
