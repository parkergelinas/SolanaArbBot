'use client';

import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';

export default function ArbitragePanel() {
  const opportunities = useMarketStore((s) => s.arbOpportunities);

  return (
    <section className="flex flex-col flex-1 min-h-0 bg-terminal-panel overflow-hidden">
      <div className="px-2 py-1 border-b border-terminal-border">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Cross-DEX Arb
        </span>
      </div>
      <ul className="flex-1 overflow-y-auto p-1 space-y-1">
        {opportunities.length === 0 ? (
          <li className="text-[11px] text-terminal-muted text-center py-8 px-2">
            Scanning spreads across Raydium · Orca · Jupiter…
          </li>
        ) : (
          opportunities.slice(0, 25).map((arb) => (
            <li
              key={arb.id}
              className="rounded border border-terminal-border bg-terminal-bg px-2 py-1.5"
            >
              <div className="flex justify-between items-center gap-2">
                <span className="mono text-[11px] text-terminal-live">
                  {tokenSymbol(arb.token)}
                </span>
                <span className="mono text-[11px] font-semibold text-flow-buy">
                  +{arb.spreadBps.toFixed(1)} bps
                </span>
              </div>
              <div className="mt-1 text-[10px] text-terminal-muted flex justify-between">
                <span>
                  Buy <span className="text-slate-300">{arb.buyDex}</span>{' '}
                  ${arb.buyPrice.toFixed(4)}
                </span>
                <span>
                  Sell <span className="text-slate-300">{arb.sellDex}</span>{' '}
                  ${arb.sellPrice.toFixed(4)}
                </span>
              </div>
            </li>
          ))
        )}
      </ul>
    </section>
  );
}
