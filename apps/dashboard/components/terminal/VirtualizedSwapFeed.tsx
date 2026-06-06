'use client';

import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useRef, useState } from 'react';

import { timeAgo } from '@/lib/format/time';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore, amountToHuman } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

const VISIBLE_ROWS = 24;
const ROW_HEIGHT = 22;

function formatAmount(mint: string, raw: string): string {
  const n = amountToHuman(mint, raw);
  if (n >= 1_000) return n.toFixed(0);
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

export default function VirtualizedSwapFeed() {
  const swaps = useMarketStore((s) => s.swaps);
  const paused = useUiStore((s) => s.swapTapePaused);
  const setPaused = useUiStore((s) => s.setSwapTapePaused);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 5_000);
    return () => clearInterval(id);
  }, []);

  const visible = swaps.slice(-VISIBLE_ROWS);
  const ordered = [...visible].reverse();
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: ordered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 6,
  });

  return (
    <section className="flex flex-col flex-shrink-0 h-44 min-h-0 border-t border-terminal-border bg-terminal-panel">
      <div className="px-2 py-1 border-b border-terminal-border flex justify-between items-center">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Live Swaps
        </span>
        <button
          type="button"
          onClick={() => setPaused(!paused)}
          className="text-[10px] text-terminal-muted hover:text-terminal-live"
        >
          {paused ? 'Resume' : 'Pause'}
        </button>
      </div>

      <div className="grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 py-0.5 text-[9px] uppercase text-terminal-muted border-b border-terminal-border shrink-0">
        <span>Time</span>
        <span>DEX</span>
        <span>Pair</span>
        <span className="text-right">In</span>
        <span className="text-right">Out</span>
      </div>

      <div ref={parentRef} className="flex-1 min-h-0 overflow-y-auto">
        {ordered.length === 0 ? (
          <div className="p-4 text-center text-[11px] text-terminal-muted">No swaps yet</div>
        ) : (
          <div
            style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}
          >
            {virtualizer.getVirtualItems().map((vRow) => {
              const s = ordered[vRow.index];
              const isBuy = !s.token_out.startsWith('So1111');
              return (
                <div
                  key={s.signature}
                  className="grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 mono text-[10px] border-b border-terminal-border/40 hover:bg-terminal-hover absolute left-0 w-full"
                  style={{
                    height: ROW_HEIGHT,
                    transform: `translateY(${vRow.start}px)`,
                  }}
                >
                  <span className="text-terminal-muted truncate leading-[22px]" title={new Date(s.timestamp_ms).toLocaleString()}>
                    {timeAgo(s.timestamp_ms, now)}
                  </span>
                  <span className="text-slate-400 truncate leading-[22px]">{s.dex}</span>
                  <span
                    className={`truncate leading-[22px] ${
                      isBuy ? 'text-flow-buy' : 'text-flow-sell'
                    }`}
                  >
                    {tokenSymbol(s.token_in)}→{tokenSymbol(s.token_out)}
                  </span>
                  <span className="text-right text-slate-500 truncate leading-[22px]">
                    {formatAmount(s.token_in, s.amount_in)}
                  </span>
                  <span className="text-right text-slate-500 truncate leading-[22px]">
                    {formatAmount(s.token_out, s.amount_out)}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
