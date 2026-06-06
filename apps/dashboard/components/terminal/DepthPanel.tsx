'use client';

import { useMemo } from 'react';

import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useUiStore } from '@/stores/uiStore';

/** Simulated L2 depth — visual reference until on-chain book feed exists. */
export default function DepthPanel() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const token = useMarketStore((s) => (selectedMint ? s.tokens[selectedMint] : undefined));
  const price = token?.price_usd ?? 1;
  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';

  const { bids, asks } = useMemo(() => {
    const levels = 8;
    const bids: { px: number; sz: number; pct: number }[] = [];
    const asks: { px: number; sz: number; pct: number }[] = [];
    let maxSz = 0;
    for (let i = 0; i < levels; i++) {
      const bSz = 200 + Math.random() * 1800;
      const aSz = 200 + Math.random() * 1800;
      maxSz = Math.max(maxSz, bSz, aSz);
      bids.push({ px: price * (1 - 0.0004 * (i + 1)), sz: bSz, pct: 0 });
      asks.push({ px: price * (1 + 0.0004 * (i + 1)), sz: aSz, pct: 0 });
    }
    for (const row of [...bids, ...asks]) row.pct = (row.sz / maxSz) * 100;
    return { bids, asks };
  }, [price, selectedMint]);

  return (
    <section className="flex flex-col min-h-0 border-b border-terminal-border bg-terminal-panel">
      <div className="px-2 py-1 border-b border-terminal-border">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Depth · {symbol}
        </span>
      </div>
      <div className="flex-1 min-h-0 overflow-hidden text-[10px] mono px-1 py-1">
        <div className="grid grid-cols-3 gap-0 text-[8px] text-terminal-muted uppercase px-1 mb-0.5">
          <span>Size</span>
          <span className="text-center">Price</span>
          <span className="text-right">Size</span>
        </div>
        {asks
          .slice()
          .reverse()
          .map((a, i) => (
            <div key={`a-${i}`} className="relative grid grid-cols-3 h-[18px] items-center px-1">
              <span />
              <span className="text-center text-flow-sell tabular-nums z-[1]">{a.px.toFixed(4)}</span>
              <span className="text-right text-slate-400 tabular-nums z-[1]">{a.sz.toFixed(0)}</span>
              <div
                className="absolute right-0 top-0 bottom-0 bg-flow-sell/10"
                style={{ width: `${a.pct}%` }}
              />
            </div>
          ))}
        <div className="text-center py-1 text-xs font-semibold text-terminal-accent border-y border-terminal-border/60 my-0.5">
          ${price.toFixed(price >= 1 ? 4 : 6)}
        </div>
        {bids.map((b, i) => (
          <div key={`b-${i}`} className="relative grid grid-cols-3 h-[18px] items-center px-1">
            <span className="text-slate-400 tabular-nums z-[1]">{b.sz.toFixed(0)}</span>
            <span className="text-center text-flow-buy tabular-nums z-[1]">{b.px.toFixed(4)}</span>
            <span />
            <div
              className="absolute left-0 top-0 bottom-0 bg-flow-buy/10"
              style={{ width: `${b.pct}%` }}
            />
          </div>
        ))}
      </div>
    </section>
  );
}
