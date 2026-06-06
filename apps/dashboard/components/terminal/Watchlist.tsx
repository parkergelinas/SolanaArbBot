'use client';

import { WATCHLIST, tokenMeta } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

function fmtPrice(price: number): string {
  if (price >= 100) return price.toFixed(2);
  if (price >= 1) return price.toFixed(4);
  if (price >= 0.01) return price.toFixed(4);
  return price.toFixed(6);
}

export default function Watchlist() {
  const tokens = useMarketStore((s) => s.tokens);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const setSelectedMint = useUiStore((s) => s.setSelectedMint);

  const rows = WATCHLIST.map((w) => {
    const live = tokens[w.mint];
    return {
      ...w,
      price: live?.price_usd ?? w.refPrice,
      changePct: live?.changePct ?? 0,
      volume: live?.volume ?? 0,
      hasLive: Boolean(live),
    };
  });

  return (
    <aside className="terminal-watchlist flex flex-col min-h-0 w-[11.5rem] shrink-0 border-r border-terminal-border bg-terminal-panel">
      <div className="px-2 py-1.5 border-b border-terminal-border flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Watchlist
        </span>
        <span className="text-[9px] text-terminal-accent">{rows.length}</span>
      </div>
      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {rows.map((row) => {
          const active = row.mint === selectedMint;
          const up = row.changePct >= 0;
          return (
            <button
              key={row.mint}
              type="button"
              onClick={() => setSelectedMint(row.mint)}
              className={`w-full text-left px-2 py-1.5 border-b border-terminal-border/40 transition-colors ${
                active ? 'bg-terminal-accent/10 border-l-2 border-l-terminal-accent' : 'hover:bg-terminal-hover border-l-2 border-l-transparent'
              }`}
            >
              <div className="flex items-center justify-between gap-1">
                <span className={`text-[11px] font-semibold ${active ? 'text-terminal-accent' : 'text-slate-200'}`}>
                  {row.symbol}
                </span>
                {!row.hasLive && (
                  <span className="text-[8px] uppercase text-terminal-muted/60">ref</span>
                )}
              </div>
              <div className="flex items-baseline justify-between mt-0.5">
                <span className="text-[11px] mono tabular-nums text-slate-100">
                  ${fmtPrice(row.price)}
                </span>
                <span className={`text-[10px] mono tabular-nums ${up ? 'text-flow-buy' : 'text-flow-sell'}`}>
                  {up ? '+' : ''}
                  {row.changePct.toFixed(2)}%
                </span>
              </div>
              <div className="text-[9px] text-terminal-muted truncate mt-0.5">{row.name}</div>
            </button>
          );
        })}
      </div>
      {selectedMint && tokenMeta(selectedMint) && (
        <div className="px-2 py-1.5 border-t border-terminal-border text-[9px] text-terminal-muted">
          Selected · {tokenMeta(selectedMint)?.symbol}/USD
        </div>
      )}
    </aside>
  );
}
