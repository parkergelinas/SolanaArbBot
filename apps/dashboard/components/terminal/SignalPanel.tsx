'use client';

import { useMemo } from 'react';

import { useMarketStore, shortMint } from '@/stores/marketStore';

const KIND_STYLE: Record<string, string> = {
  momentum: 'text-terminal-live',
  whale_flow: 'text-violet-400',
  smart_money: 'text-purple-400',
  imbalance: 'text-amber-400',
};

export default function SignalPanel() {
  const signals = useMarketStore((s) => s.signals);

  const visible = useMemo(() => {
    const seen = new Set<string>();
    const out: typeof signals = [];
    for (const sig of signals) {
      const key = `${sig.kind}:${sig.mint}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(sig);
      if (out.length >= 30) break;
    }
    return out;
  }, [signals]);

  return (
    <aside className="w-52 flex-shrink-0 flex flex-col h-full min-h-0 bg-terminal-panel border-l border-terminal-border">
      <div className="px-2 py-1 border-b border-terminal-border">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Signals
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto p-1 space-y-1 min-h-0">
        {visible.length === 0 ? (
          <li className="text-[10px] text-terminal-muted text-center py-6">No signals</li>
        ) : (
          visible.map((sig) => (
            <li
              key={`${sig.signal_id}-${sig.timestamp_ms}`}
              className="border border-terminal-border rounded px-2 py-1 bg-terminal-bg"
            >
              <div className="flex justify-between text-[10px]">
                <span className={KIND_STYLE[sig.kind] ?? 'text-slate-300'}>
                  {sig.kind.replace('_', ' ')}
                </span>
                <span className="text-terminal-muted">{(sig.confidence * 100).toFixed(0)}%</span>
              </div>
              <div className="mono text-[10px] text-terminal-live mt-0.5">
                {shortMint(sig.mint)}
              </div>
              {sig.detail && (
                <div className="text-[9px] text-terminal-muted mt-0.5 truncate">{sig.detail}</div>
              )}
            </li>
          ))
        )}
      </ul>
    </aside>
  );
}
