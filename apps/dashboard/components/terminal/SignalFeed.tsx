'use client';

import Link from 'next/link';
import { memo, useMemo, useState } from 'react';

import type { Signal } from '@/lib/stream/types';
import { formatPumpDetail, isPumpSignal } from '@/lib/terminal/pumpSignals';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { selectSignals } from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';

type FilterTab = 'all' | 'pump' | 'momentum' | 'whale' | 'imbalance';

const TABS: { key: FilterTab; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'pump', label: 'Pump' },
  { key: 'momentum', label: 'Mom' },
  { key: 'whale', label: 'Whale' },
  { key: 'imbalance', label: 'Imb' },
];

function matchesFilter(sig: Signal, tab: FilterTab): boolean {
  if (tab === 'all') return true;
  if (tab === 'pump') return isPumpSignal(sig);
  if (tab === 'momentum') return sig.kind === 'momentum' && !isPumpSignal(sig);
  if (tab === 'whale') return sig.kind === 'whale_flow' || sig.kind === 'smart_money';
  return sig.kind === 'imbalance';
}

const SignalItem = memo(function SignalItem({
  kind,
  confidence,
  mint,
  detail,
}: {
  kind: string;
  confidence: number;
  mint: string;
  detail?: string;
}) {
  const pump = detail?.startsWith('pump_');
  const pumpKind = detail?.startsWith('pump_launch')
    ? 'launch'
    : detail?.startsWith('pump_curve')
      ? 'curve'
      : null;

  return (
    <li
      className={`px-3 py-2 border-b border-ds-border hover:bg-ds-elevated/60 transition-colors ${
        pump ? 'border-l-2 border-l-emerald-500/60' : ''
      }`}
    >
      <div className="flex justify-between text-[10px]">
        <span className={`uppercase ${pump ? 'text-emerald-400 font-semibold' : 'text-ds-text-secondary'}`}>
          {pump ? (pumpKind === 'launch' ? 'pump launch' : 'pump curve') : kind.replace('_', ' ')}
        </span>
        <span className="font-mono text-ds-text-muted">{(confidence * 100).toFixed(0)}%</span>
      </div>
      <div className="font-mono text-[11px] text-ds-text-primary mt-0.5">{tokenSymbol(mint)}</div>
      {detail && (
        <div className="text-[10px] text-ds-text-muted mt-0.5 truncate">
          {pump ? formatPumpDetail(detail) : detail}
        </div>
      )}
    </li>
  );
});

export default function SignalFeed() {
  const signals = useMarketStore(selectSignals);
  const [tab, setTab] = useState<FilterTab>('all');

  const visible = useMemo(() => {
    const seen = new Set<string>();
    const out: Signal[] = [];
    for (const sig of signals) {
      if (!matchesFilter(sig, tab)) continue;
      const key = `${sig.signal_id}:${sig.mint}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(sig);
      if (out.length >= 40) break;
    }
    return out;
  }, [signals, tab]);

  const pumpCount = useMemo(() => signals.filter(isPumpSignal).length, [signals]);

  return (
    <div className="flex flex-col h-full min-h-0">
      <div className="flex items-center justify-between px-2 py-1 border-b border-ds-border shrink-0">
        <div className="flex gap-1 flex-wrap">
          {TABS.map((t) => (
            <button
              key={t.key}
              type="button"
              onClick={() => setTab(t.key)}
              className={`text-[9px] px-1.5 py-0.5 rounded-terminal border font-mono uppercase tracking-wide ${
                tab === t.key
                  ? t.key === 'pump'
                    ? 'border-emerald-500/40 text-emerald-400 bg-emerald-500/10'
                    : 'border-ds-accent/40 text-ds-accent bg-ds-accent/8'
                  : 'border-ds-border text-ds-text-muted hover:text-ds-text-secondary'
              }`}
            >
              {t.label}
              {t.key === 'pump' && pumpCount > 0 ? ` ${pumpCount}` : ''}
            </button>
          ))}
        </div>
        <Link
          href="/signals"
          className="text-[9px] text-ds-text-muted hover:text-ds-accent shrink-0"
        >
          More
        </Link>
      </div>

      <ul className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {visible.length === 0 ? (
          <li className="px-3 py-6 text-center text-[10px] text-ds-text-muted leading-relaxed">
            {tab === 'pump'
              ? 'No pump.fun signals yet — enable HELIUS key on stream-api for live launches.'
              : 'Waiting for stream signals…'}
          </li>
        ) : (
          visible.map((sig) => (
            <SignalItem
              key={`${sig.signal_id}-${sig.timestamp_ms}`}
              kind={sig.kind}
              confidence={sig.confidence}
              mint={sig.mint}
              detail={sig.detail}
            />
          ))
        )}
      </ul>
    </div>
  );
}
