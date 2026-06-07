'use client';

import Link from 'next/link';
import { memo, useMemo, useState } from 'react';

import type { Signal } from '@/lib/stream/types';
import { formatPumpDetail, isPumpSignal } from '@/lib/terminal/pumpSignals';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { selectSignals } from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';

// ── Filter types ───────────────────────────────────────────────────────────────
type FilterTab = 'all' | 'pump' | 'momentum' | 'whale' | 'imbalance' | 'arb';
type TimeBucket = '1m' | '5m' | '15m' | 'all';
type ConfThreshold = 50 | 60 | 70 | 80 | 90;

const TABS: { key: FilterTab; label: string; color?: string }[] = [
  { key: 'all',       label: 'All' },
  { key: 'pump',      label: 'Pump',  color: 'emerald' },
  { key: 'momentum',  label: 'Mom' },
  { key: 'whale',     label: 'Whale', color: 'blue' },
  { key: 'imbalance', label: 'Imb' },
  { key: 'arb',       label: 'Arb',   color: 'amber' },
];

const TIME_BUCKETS: { key: TimeBucket; label: string; ms: number }[] = [
  { key: '1m',  label: '1m',  ms: 60_000 },
  { key: '5m',  label: '5m',  ms: 300_000 },
  { key: '15m', label: '15m', ms: 900_000 },
  { key: 'all', label: '∞',   ms: Infinity },
];

const CONF_STEPS: ConfThreshold[] = [50, 60, 70, 80, 90];

// ── Helpers ────────────────────────────────────────────────────────────────────
function matchesTab(sig: Signal, tab: FilterTab): boolean {
  if (tab === 'all') return true;
  if (tab === 'pump') return isPumpSignal(sig);
  if (tab === 'momentum') return sig.kind === 'momentum' && !isPumpSignal(sig);
  if (tab === 'whale') return sig.kind === 'whale_flow' || sig.kind === 'smart_money';
  if (tab === 'arb') return sig.kind === 'arb' || sig.kind === 'route_divergence';
  return sig.kind === 'imbalance';
}

function ageMs(sig: Signal): number {
  return Date.now() - (sig.timestamp_ms ?? 0);
}

function ageLabel(ms: number): string {
  if (ms < 60_000) return `${Math.floor(ms / 1000)}s`;
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m`;
  return `${Math.floor(ms / 3_600_000)}h`;
}

// ── Signal row ─────────────────────────────────────────────────────────────────
const SignalItem = memo(function SignalItem({
  sig,
  isNew,
}: {
  sig: Signal;
  isNew: boolean;
}) {
  const pump = isPumpSignal(sig);
  const pumpKind = sig.detail?.startsWith('pump_launch')
    ? 'launch'
    : sig.detail?.startsWith('pump_curve')
      ? 'curve'
      : null;
  const isArb = sig.kind === 'arb' || sig.kind === 'route_divergence';
  const isWhale = sig.kind === 'whale_flow' || sig.kind === 'smart_money';
  const confPct = Math.round((sig.confidence ?? 0) * 100);
  const age = ageMs(sig);

  const borderColor = pump
    ? 'border-l-emerald-500/70'
    : isArb
      ? 'border-l-ds-amber/70'
      : isWhale
        ? 'border-l-ds-blue/70'
        : 'border-l-transparent';

  const kindLabel = pump
    ? pumpKind === 'launch'
      ? 'pump launch'
      : 'pump curve'
    : sig.kind.replace(/_/g, ' ');

  const kindColor = pump
    ? 'text-emerald-400'
    : isArb
      ? 'text-ds-amber'
      : isWhale
        ? 'text-ds-blue'
        : 'text-ds-text-secondary';

  return (
    <li
      className={`px-2.5 py-1.5 border-b border-ds-border hover:bg-ds-elevated/60 transition-colors border-l-2 ${borderColor} ${
        isNew ? 'bg-ds-blue/5' : ''
      }`}
    >
      <div className="flex items-center justify-between gap-2 text-[10px]">
        <span className={`uppercase font-medium shrink-0 ${kindColor}`}>{kindLabel}</span>
        <div className="flex items-center gap-1.5 shrink-0 font-mono text-ds-text-muted">
          {isNew && (
            <span className="text-[8px] px-1 py-px bg-ds-blue/20 text-ds-blue border border-ds-blue/30 rounded uppercase">
              new
            </span>
          )}
          <span className={confPct >= 80 ? 'text-ds-green' : confPct >= 60 ? 'text-ds-text-secondary' : 'text-ds-text-muted'}>
            {confPct}%
          </span>
          <span className="text-[9px] text-ds-text-muted">{ageLabel(age)}</span>
        </div>
      </div>
      <div className="font-mono text-[11px] text-ds-text-primary mt-0.5">
        {tokenSymbol(sig.mint)}
      </div>
      {sig.detail && (
        <div className="text-[9px] text-ds-text-muted mt-0.5 truncate">
          {pump ? formatPumpDetail(sig.detail) : sig.detail}
        </div>
      )}
    </li>
  );
});

// ── Main component ─────────────────────────────────────────────────────────────
export default function SignalFeed() {
  const signals = useMarketStore(selectSignals);
  const [tab, setTab] = useState<FilterTab>('all');
  const [timeBucket, setTimeBucket] = useState<TimeBucket>('all');
  const [confThreshold, setConfThreshold] = useState<ConfThreshold>(50);

  const bucketMs = TIME_BUCKETS.find((b) => b.key === timeBucket)?.ms ?? Infinity;
  const now = Date.now();

  // Count per tab (before time/conf filters so tabs always show realistic counts)
  const tabCounts = useMemo(() => {
    const counts: Record<FilterTab, number> = {
      all: 0, pump: 0, momentum: 0, whale: 0, imbalance: 0, arb: 0,
    };
    for (const sig of signals) {
      for (const t of TABS) {
        if (matchesTab(sig, t.key)) counts[t.key]++;
      }
    }
    return counts;
  }, [signals]);

  const visible = useMemo(() => {
    const seen = new Set<string>();
    const out: Signal[] = [];
    for (const sig of signals) {
      if (!matchesTab(sig, tab)) continue;
      if ((sig.confidence ?? 0) < confThreshold / 100) continue;
      if (bucketMs < Infinity && now - (sig.timestamp_ms ?? 0) > bucketMs) continue;
      const key = `${sig.signal_id}:${sig.mint}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(sig);
      if (out.length >= 50) break;
    }
    return out;
  }, [signals, tab, confThreshold, bucketMs, now]);

  const NEW_THRESHOLD_MS = 12_000;

  return (
    <div className="flex flex-col h-full min-h-0">
      {/* ── Tab bar ──────────────────────────────────────────────────────────── */}
      <div className="flex items-center justify-between px-2 pt-1 pb-0.5 border-b border-ds-border shrink-0 gap-1">
        <div className="flex gap-0.5 flex-wrap">
          {TABS.map((t) => {
            const count = tabCounts[t.key];
            const isActive = tab === t.key;
            const isEmerald = t.color === 'emerald';
            const isBlue = t.color === 'blue';
            const isAmber = t.color === 'amber';
            return (
              <button
                key={t.key}
                type="button"
                onClick={() => setTab(t.key)}
                className={`text-[9px] px-1.5 py-0.5 rounded-terminal border font-mono uppercase tracking-wide transition-colors ${
                  isActive
                    ? isEmerald
                      ? 'border-emerald-500/40 text-emerald-400 bg-emerald-500/10'
                      : isBlue
                        ? 'border-ds-blue/40 text-ds-blue bg-ds-blue/10'
                        : isAmber
                          ? 'border-ds-amber/40 text-ds-amber bg-ds-amber/10'
                          : 'border-ds-accent/40 text-ds-accent bg-ds-accent/8'
                    : 'border-ds-border text-ds-text-muted hover:text-ds-text-secondary'
                }`}
              >
                {t.label}
                {count > 0 && (
                  <span className={`ml-0.5 ${isActive ? '' : 'opacity-60'}`}>{count}</span>
                )}
              </button>
            );
          })}
        </div>
        <Link href="/signals" className="text-[9px] text-ds-text-muted hover:text-ds-accent shrink-0">
          More →
        </Link>
      </div>

      {/* ── Filter bar ───────────────────────────────────────────────────────── */}
      <div className="terminal-filterbar shrink-0">
        {/* Confidence threshold */}
        <span className="text-ds-text-muted shrink-0">Conf≥</span>
        {CONF_STEPS.map((c) => (
          <button
            key={c}
            type="button"
            onClick={() => setConfThreshold(c)}
            className={`filter-pill ${confThreshold === c ? 'active' : ''}`}
          >
            {c}%
          </button>
        ))}

        {/* Divider */}
        <span className="w-px h-3.5 bg-ds-border shrink-0 mx-0.5" />

        {/* Time bucket */}
        <span className="text-ds-text-muted shrink-0">Age</span>
        {TIME_BUCKETS.map((b) => (
          <button
            key={b.key}
            type="button"
            onClick={() => setTimeBucket(b.key)}
            className={`filter-pill ${timeBucket === b.key ? 'active' : ''}`}
          >
            {b.label}
          </button>
        ))}

        {/* Count */}
        <span className="ml-auto text-ds-text-muted tabular-nums shrink-0">
          {visible.length}
        </span>
      </div>

      {/* ── Signal list ──────────────────────────────────────────────────────── */}
      <ul className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {visible.length === 0 ? (
          <li className="px-3 py-6 text-center text-[10px] text-ds-text-muted leading-relaxed">
            {tab === 'pump'
              ? 'No pump.fun signals — enable HELIUS key on stream-api for live launches.'
              : confThreshold > 50
                ? `No signals above ${confThreshold}% confidence.`
                : 'Waiting for stream signals…'}
          </li>
        ) : (
          visible.map((sig) => (
            <SignalItem
              key={`${sig.signal_id}-${sig.timestamp_ms}`}
              sig={sig}
              isNew={ageMs(sig) < NEW_THRESHOLD_MS}
            />
          ))
        )}
      </ul>
    </div>
  );
}
