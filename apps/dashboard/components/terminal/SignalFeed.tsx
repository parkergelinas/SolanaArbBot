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
  { key: 'momentum', label: 'Mom' },
  { key: 'whale', label: 'Whale' },
  { key: 'imbalance', label: 'Imb' },
];

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
    <li className="px-3 py-2 border-b border-ds-border hover:bg-ds-elevated/60 transition-colors">
      <div className="flex justify-between text-[10px]">
        <span className="text-ds-text-secondary uppercase">{kind.replace('_', ' ')}</span>
        <span className="font-mono text-ds-text-muted">{(confidence * 100).toFixed(0)}%</span>
      </div>
      <div className="font-mono text-[11px] text-ds-text-primary mt-0.5">{tokenSymbol(mint)}</div>
      {detail && (
        <div className="text-[10px] text-ds-text-muted mt-0.5 truncate">{detail}</div>
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
      if (!matchesFilter(sig.kind, tab)) continue;
      const key = `${sig.kind}:${sig.mint}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(sig);
      if (out.length >= 40) break;
    }
    return out;
  }, [signals, tab]);

  return (
    <div className="flex flex-col h-full min-h-0">
      <div className="flex gap-0 border-b border-ds-border shrink-0 px-1 py-1">
        {TABS.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            className={`px-2 py-0.5 text-[9px] uppercase tracking-wider rounded-terminal transition-colors ${
              tab === key
                ? 'bg-ds-elevated text-ds-blue border border-ds-blue/30'
                : 'text-ds-text-muted hover:text-ds-text-secondary'
            }`}
          >
            {label}
          </button>
        ))}
        <span className="ml-auto font-mono text-[10px] text-ds-text-muted pr-2 self-center">
          {visible.length}
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto terminal-scroll min-h-0">
        {visible.length === 0 ? (
          <li className="text-[11px] text-ds-text-muted text-center py-8">No signals yet</li>
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
      <Link
        href="/signals"
        className="block text-center py-2 text-[10px] text-ds-blue hover:underline border-t border-ds-border shrink-0 uppercase tracking-wider"
      >
        Full feed →
      </Link>
    </div>
  );
}
