'use client';

import { useSmartMoney, useWhales } from '@/lib/hooks';

function shortWallet(w: string) {
  return w.length > 12 ? `${w.slice(0, 6)}…${w.slice(-4)}` : w;
}

export default function WhaleFeed() {
  const whales = useWhales();
  const smart = useSmartMoney();

  return (
    <section className="flex-1 flex flex-col min-h-0 bg-platform-bg border border-platform-border rounded-xl overflow-hidden">
      <div className="px-3 py-2 border-b border-platform-border bg-platform-surface">
        <span className="text-[10px] uppercase tracking-widest text-platform-muted font-medium">
          Whale & Smart Money Alerts
        </span>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {whales.length === 0 && smart.length === 0 ? (
          <p className="text-xs text-platform-muted text-center py-8">
            Scanning for whale activity…
          </p>
        ) : (
          <>
            {whales.map((w) => (
              <article
                key={w.alert_id}
                className="rounded-lg border border-platform-accent/30 bg-platform-accent/5 px-3 py-2"
              >
                <div className="flex justify-between items-start">
                  <span className="text-[10px] font-semibold text-platform-accent uppercase">
                    Whale · {w.dex}
                  </span>
                  <span className="text-[10px] mono text-platform-muted">
                    {(w.strength * 100).toFixed(0)}%
                  </span>
                </div>
                <p className="text-lg font-semibold mono text-slate-100 mt-1">
                  {w.amount_sol.toFixed(2)} SOL
                  <span className="text-sm text-platform-muted ml-2">
                    ${w.notional_usd.toFixed(0)}
                  </span>
                </p>
                <p className="text-[10px] mono text-platform-muted mt-0.5">
                  {w.token_symbol} · {shortWallet(w.wallet)}
                </p>
              </article>
            ))}
            {smart.map((s) => (
              <article
                key={s.alert_id}
                className="rounded-lg border border-purple-500/30 bg-purple-500/5 px-3 py-2"
              >
                <div className="flex justify-between">
                  <span className="text-[10px] font-semibold text-purple-400 uppercase">
                    Smart $
                  </span>
                  <span className="text-[10px] mono text-platform-muted">
                    {(s.confidence * 100).toFixed(0)}%
                  </span>
                </div>
                <p className="text-sm font-medium mono text-slate-200 mt-1">
                  {s.amount_sol.toFixed(2)} SOL · {s.token_symbol}
                </p>
                <p className="text-[10px] text-platform-muted">{shortWallet(s.wallet)}</p>
              </article>
            ))}
          </>
        )}
      </div>
    </section>
  );
}
