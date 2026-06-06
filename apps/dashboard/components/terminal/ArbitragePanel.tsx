'use client';

import { formatPrice } from '@/lib/formatters';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';

export default function ArbitragePanel() {
  const opportunities = useMarketStore((s) => s.arbOpportunities);

  return (
    <section className="flex flex-col h-full min-h-0 bg-ds-surface overflow-hidden">
      <div className="flex items-center justify-between h-8 px-3 border-b border-ds-border shrink-0">
        <span className="text-[11px] font-medium uppercase tracking-[0.1em] text-ds-text-secondary">
          Cross-DEX Arb
        </span>
        <span className="text-[10px] font-mono text-ds-text-muted">{opportunities.length}</span>
      </div>
      <ul className="flex-1 overflow-y-auto terminal-scroll min-h-0">
        {opportunities.length === 0 ? (
          <li className="text-[11px] text-ds-text-muted text-center py-10 px-3">
            Scanning Raydium · Orca · Jupiter…
          </li>
        ) : (
          opportunities.slice(0, 30).map((arb) => (
            <li
              key={arb.id}
              className="px-3 py-2 border-b border-ds-border hover:bg-ds-elevated/50 transition-colors"
            >
              <div className="flex justify-between items-center gap-2">
                <span className="font-mono text-[11px] text-ds-text-primary">
                  {tokenSymbol(arb.token)}
                </span>
                <span className="font-mono text-[11px] font-medium text-ds-green">
                  +{arb.spreadBps.toFixed(1)} bps
                </span>
              </div>
              <div className="mt-1 text-[10px] text-ds-text-muted flex justify-between font-mono">
                <span>
                  Buy <span className="text-ds-text-secondary">{arb.buyDex}</span>{' '}
                  {formatPrice(arb.buyPrice)}
                </span>
                <span>
                  Sell <span className="text-ds-text-secondary">{arb.sellDex}</span>{' '}
                  {formatPrice(arb.sellPrice)}
                </span>
              </div>
            </li>
          ))
        )}
      </ul>
    </section>
  );
}
