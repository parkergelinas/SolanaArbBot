'use client';

import { useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';
import { tierColor, tierLabel } from '@/lib/intelligence/whaleSources';

export default function WhaleFeed({ compact = false }: { compact?: boolean }) {
  const whales = useWhales();
  const smart = useSmartMoney();

  return (
    <section className="bg-ds-surface border border-ds-border rounded-terminal flex flex-col min-h-0 overflow-hidden">
      <div className="px-3 py-2.5 border-b border-ds-border flex items-center justify-between">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Whale Radar
        </span>
        <span className="text-[10px] font-mono text-ds-blue">
          {whales.length + smart.length} alerts
        </span>
      </div>
      <div className={`flex-1 overflow-y-auto p-2 space-y-2 ${compact ? 'max-h-72' : 'min-h-[200px]'}`}>
        {whales.length === 0 && smart.length === 0 ? (
          <p className="text-xs text-ds-text-muted text-center py-8">
            Scanning chain for whale activity…
            <span className="block text-[10px] mt-1 font-mono">intelligence-api :8090</span>
          </p>
        ) : (
          <>
            {whales.map((w) => (
              <article
                key={w.alert_id}
                className="rounded-terminal border border-ds-blue/25 bg-ds-blue/5 px-3 py-2.5"
              >
                <div className="flex justify-between items-start gap-2">
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <span className="text-[10px] font-bold text-ds-blue uppercase tracking-wide">
                      Whale · {w.dex}
                    </span>
                    <span
                      className="text-[9px] font-mono px-1 py-px border rounded-terminal"
                      style={{ color: tierColor(w.tier), borderColor: `${tierColor(w.tier)}40` }}
                    >
                      {tierLabel(w.tier)}
                    </span>
                  </div>
                  <span className="text-[10px] font-mono text-ds-text-muted shrink-0">
                    {(w.confidence * 100).toFixed(0)}%
                  </span>
                </div>
                <p className="text-lg font-semibold font-mono text-ds-text-primary mt-1">
                  {w.amount_sol.toFixed(2)} SOL
                  <span className="text-sm text-ds-text-muted ml-2 font-normal">
                    ${w.notional_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                  </span>
                </p>
                <p className="text-[10px] font-mono text-ds-text-muted mt-0.5">
                  {w.token_symbol} · {shortPool(w.wallet, 4, 4)}
                </p>
                {w.detail && (
                  <p className="text-[10px] text-ds-text-secondary mt-1 leading-snug">{w.detail}</p>
                )}
              </article>
            ))}
            {smart.map((s) => (
              <article
                key={s.alert_id}
                className="rounded-terminal border border-purple-500/25 bg-purple-500/5 px-3 py-2.5"
              >
                <div className="flex justify-between">
                  <span className="text-[10px] font-bold text-purple-400 uppercase tracking-wide">
                    Smart $
                  </span>
                  <span className="text-[10px] font-mono text-ds-text-muted">
                    {(s.strength * 100).toFixed(0)}% str
                  </span>
                </div>
                <p className="text-sm font-medium font-mono text-ds-text-primary mt-1">
                  {s.amount_sol.toFixed(2)} SOL · {s.token_symbol}
                </p>
                <p className="text-[10px] text-ds-text-muted font-mono">{shortPool(s.wallet, 4, 4)}</p>
              </article>
            ))}
          </>
        )}
      </div>
    </section>
  );
}
