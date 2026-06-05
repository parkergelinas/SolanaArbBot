'use client';

import { useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';

export default function WhaleFeed({ compact = false }: { compact?: boolean }) {
  const whales = useWhales();
  const smart = useSmartMoney();

  return (
    <section className="glass-card flex flex-col min-h-0 overflow-hidden">
      <div className="px-3 py-2.5 border-b border-platform-border/80 flex items-center justify-between">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted font-semibold">
          Whale Radar
        </span>
        <span className="text-[10px] mono text-platform-accent">
          {whales.length + smart.length} alerts
        </span>
      </div>
      <div className={`flex-1 overflow-y-auto p-2 space-y-2 ${compact ? 'max-h-72' : 'min-h-[200px]'}`}>
        {whales.length === 0 && smart.length === 0 ? (
          <p className="text-xs text-platform-muted text-center py-8">
            Scanning chain for whale activity…
          </p>
        ) : (
          <>
            {whales.map((w) => (
              <article
                key={w.alert_id}
                className="rounded-lg border border-platform-accent/25 bg-gradient-to-br from-platform-accent/10 to-transparent px-3 py-2.5"
              >
                <div className="flex justify-between items-start gap-2">
                  <span className="text-[10px] font-bold text-platform-accent uppercase tracking-wide">
                    Whale · {w.dex}
                  </span>
                  <span className="text-[10px] mono text-platform-muted shrink-0">
                    {(w.confidence * 100).toFixed(0)}% conf
                  </span>
                </div>
                <p className="text-lg font-semibold mono text-slate-100 mt-1">
                  {w.amount_sol.toFixed(2)} SOL
                  <span className="text-sm text-platform-muted ml-2 font-normal">
                    ${w.notional_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                  </span>
                </p>
                <p className="text-[10px] mono text-platform-muted mt-0.5">
                  {w.token_symbol} · {shortPool(w.wallet, 4, 4)}
                </p>
              </article>
            ))}
            {smart.map((s) => (
              <article
                key={s.alert_id}
                className="rounded-lg border border-purple-500/25 bg-gradient-to-br from-purple-500/10 to-transparent px-3 py-2.5"
              >
                <div className="flex justify-between">
                  <span className="text-[10px] font-bold text-purple-400 uppercase tracking-wide">
                    Smart $
                  </span>
                  <span className="text-[10px] mono text-platform-muted">
                    {(s.strength * 100).toFixed(0)}% str
                  </span>
                </div>
                <p className="text-sm font-medium mono text-slate-200 mt-1">
                  {s.amount_sol.toFixed(2)} SOL · {s.token_symbol}
                </p>
                <p className="text-[10px] text-platform-muted mono">{shortPool(s.wallet, 4, 4)}</p>
              </article>
            ))}
          </>
        )}
      </div>
    </section>
  );
}
