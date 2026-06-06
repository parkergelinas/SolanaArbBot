'use client';

import { formatPrice } from '@/lib/formatters';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';

function sourceLabel(source?: string): string {
  if (source === 'dexscreener') return 'DS';
  if (source === 'jupiter') return 'JUP';
  return 'WS';
}

export default function ArbitragePanel() {
  const opportunities = useMarketStore((s) => s.arbOpportunities);

  return (
    <section className="flex flex-col h-full min-h-0 bg-ds-surface overflow-hidden">
      <div className="flex items-center justify-between h-8 px-3 border-b border-ds-border shrink-0">
        <span className="text-[11px] font-medium uppercase tracking-[0.1em] text-ds-text-secondary">
          Cross-DEX Arb
        </span>
        <span className="text-[10px] font-mono text-ds-text-muted">{opportunities.length} live</span>
      </div>
      <ul className="flex-1 overflow-y-auto terminal-scroll min-h-0">
        {opportunities.length === 0 ? (
          <li className="text-[11px] text-ds-text-muted text-center py-10 px-3 leading-relaxed">
            Scanning Raydium · Orca · Jupiter · DexScreener…
            <br />
            <span className="text-[10px]">Requires stream-api swaps or DexScreener pairs</span>
          </li>
        ) : (
          opportunities.slice(0, 30).map((arb) => (
            <li
              key={arb.id}
              className="px-3 py-2 border-b border-ds-border hover:bg-ds-elevated/50 transition-colors"
            >
              <div className="flex justify-between items-center gap-2">
                <div className="flex items-center gap-1.5 min-w-0">
                  <span className="font-mono text-[11px] text-ds-text-primary truncate">
                    {arb.symbol ?? tokenSymbol(arb.token)}
                  </span>
                  <span className="text-[8px] px-1 py-0.5 rounded border border-ds-border text-ds-text-muted font-mono shrink-0">
                    {sourceLabel(arb.source)}
                  </span>
                </div>
                <span className="font-mono text-[11px] font-medium text-ds-green shrink-0">
                  +{arb.spreadBps.toFixed(1)} bps
                </span>
              </div>
              <div className="mt-1 text-[10px] text-ds-text-muted flex justify-between font-mono gap-2">
                <span className="truncate">
                  Buy <span className="text-ds-text-secondary">{arb.buyDexLabel ?? arb.buyDex}</span>{' '}
                  {formatPrice(arb.buyPrice)}
                </span>
                <span className="truncate text-right">
                  Sell <span className="text-ds-text-secondary">{arb.sellDexLabel ?? arb.sellDex}</span>{' '}
                  {formatPrice(arb.sellPrice)}
                </span>
              </div>
              <div className="mt-1 flex justify-between text-[9px] font-mono text-ds-text-muted">
                <span>
                  Est{' '}
                  <span className={arb.estimatedProfitUsd && arb.estimatedProfitUsd > 0 ? 'text-ds-green' : ''}>
                    ${(arb.estimatedProfitUsd ?? 0).toFixed(2)}
                  </span>
                  {arb.venueCount != null && (
                    <span className="text-ds-text-muted"> · {arb.venueCount} venues</span>
                  )}
                </span>
                {arb.winProbability != null && (
                  <span className="text-ds-blue">{(arb.winProbability * 100).toFixed(0)}% fill</span>
                )}
              </div>
            </li>
          ))
        )}
      </ul>
    </section>
  );
}
