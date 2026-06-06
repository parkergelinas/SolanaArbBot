'use client';

import Link from 'next/link';
import { memo, useMemo, useState } from 'react';

import type { Signal, SignalKind } from '@/lib/stream/types';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { selectSignals } from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';

type FilterTab = 'all' | 'momentum' | 'whale' | 'imbalance';

const TABS: { key: FilterTab; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'momentum', label: 'Momentum' },
  { key: 'whale', label: 'Whale' },
  { key: 'imbalance', label: 'Imbalance' },
];

const KIND_STYLE: Record<string, string> = {
  momentum: 'text-terminal-live',
  whale_flow: 'text-violet-400',
  smart_money: 'text-purple-400',
  imbalance: 'text-amber-400',
};

function matchesFilter(kind: SignalKind, tab: FilterTab): boolean {
  if (tab === 'all') return true;
  if (tab === 'momentum') return kind === 'momentum';
  if (tab === 'whale') return kind === 'whale_flow' || kind === 'smart_money';
  return kind === 'imbalance';
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
  return (
    <li className="border border-terminal-border rounded px-2 py-1 bg-terminal-bg">
      <div className="flex justify-between text-[10px]">
        <span className={KIND_STYLE[kind] ?? 'text-slate-300'}>
          {kind.replace('_', ' ')}
        </span>
        <span className="text-terminal-muted">{(confidence * 100).toFixed(0)}%</span>
      </div>
      <div className="mono text-[10px] text-terminal-live mt-0.5">{tokenSymbol(mint)}</div>
      {detail && (
        <div className="text-[9px] text-terminal-muted mt-0.5 truncate">{detail}</div>
      )}
    </li>
  );
});

export default function SignalPanel() {
  const signals = useMarketStore(selectSignals);
  const [tab, setTab] = useState<FilterTab>('all');

  const visible = useMemo(() => {
    const seen = new Set<string>();
    const out: Signal[] = [];
    for (const sig of signals) {
      if (!matchesFilter(sig.kind, tab)) continue;
      const key = `${sig.kind}:${sig.mint}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(sig);
      if (out.length >= 30) break;
    }
    return out;
  }, [signals, tab]);

  return (
    <aside className="w-52 flex-shrink-0 flex flex-col h-full min-h-0 bg-terminal-panel border-l border-terminal-border">
      <div className="px-2 py-1 border-b border-terminal-border flex items-center justify-between gap-1">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Signals
        </span>
        <span className="text-[9px] mono text-terminal-accent">{visible.length}</span>
      </div>
      <div className="flex flex-wrap gap-0.5 px-1 py-1 border-b border-terminal-border">
        {TABS.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            className={`px-1.5 py-0.5 rounded text-[8px] uppercase tracking-wide transition-colors ${
              tab === key
                ? 'bg-terminal-accent/15 text-terminal-accent border border-terminal-accent/30'
                : 'text-terminal-muted hover:text-slate-300 border border-transparent'
            }`}
          >
            {label}
          </button>
        ))}
      </div>
      <ul className="flex-1 overflow-y-auto p-1 space-y-1 min-h-0">
        {visible.length === 0 ? (
          <li className="text-[10px] text-terminal-muted text-center py-6">No signals</li>
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
      <div className="px-2 py-1 border-t border-terminal-border">
        <Link
          href="/signals"
          className="block text-center text-[9px] text-terminal-accent hover:text-terminal-live uppercase tracking-wider"
        >
          Full feed →
        </Link>
      </div>
    </aside>
  );
}
