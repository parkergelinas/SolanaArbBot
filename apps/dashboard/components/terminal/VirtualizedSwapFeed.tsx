'use client';

import { useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';

import { useMarketStore, shortMint } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

const ROW_H = 22;

export default function VirtualizedSwapFeed() {
  const swaps = useMarketStore((s) => s.swaps);
  const paused = useUiStore((s) => s.swapTapePaused);
  const setPaused = useUiStore((s) => s.setSwapTapePaused);

  const visible = paused ? swaps.slice(-80) : swaps;
  const ordered = [...visible].reverse();

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: ordered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_H,
    overscan: 12,
  });

  return (
    <section className="flex flex-col min-h-0 border-t border-terminal-border bg-terminal-panel">
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

      <div className="grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 py-0.5 text-[9px] uppercase text-terminal-muted border-b border-terminal-border">
        <span>Time</span>
        <span>DEX</span>
        <span>Pair</span>
        <span className="text-right">In</span>
        <span className="text-right">Out</span>
      </div>

      <div ref={parentRef} className="flex-1 overflow-auto min-h-[140px]">
        {ordered.length === 0 ? (
          <div className="p-4 text-center text-[11px] text-terminal-muted">No swaps yet</div>
        ) : (
          <div
            style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}
          >
            {virtualizer.getVirtualItems().map((vRow) => {
              const s = ordered[vRow.index];
              const isBuy = s.token_out.startsWith('So1111') === false;
              return (
                <div
                  key={s.signature}
                  className="absolute left-0 right-0 grid grid-cols-[3.5rem_3rem_1fr_4.5rem_4.5rem] gap-0 px-2 mono text-[10px] border-b border-terminal-border/40 hover:bg-terminal-hover"
                  style={{
                    height: ROW_H,
                    transform: `translateY(${vRow.start}px)`,
                  }}
                >
                  <span className="text-terminal-muted truncate leading-[22px]">
                    {new Date(s.timestamp_ms).toLocaleTimeString()}
                  </span>
                  <span className="text-slate-400 truncate leading-[22px]">{s.dex}</span>
                  <span
                    className={`truncate leading-[22px] ${
                      isBuy ? 'text-flow-buy' : 'text-flow-sell'
                    }`}
                  >
                    {shortMint(s.token_in, 3, 2)}→{shortMint(s.token_out, 3, 2)}
                  </span>
                  <span className="text-right text-slate-500 leading-[22px]">{s.amount_in}</span>
                  <span className="text-right text-slate-500 leading-[22px]">{s.amount_out}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
