'use client';

import { useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { shortPool } from '@/lib/signals';
import { tierColor, tierLabel } from '@/lib/intelligence/whaleSources';

export default function WhaleFeed({
  compact = false,
  fillHeight = false,
}: {
  compact?: boolean;
  fillHeight?: boolean;
}) {
  const whales = useWhales();
  const smart = useSmartMoney();

  return (
    <section
      className={`bg-ds-surface flex flex-col min-h-0 overflow-hidden ${
        fillHeight ? 'h-full rounded-none border-0' : 'border border-ds-border rounded-terminal'
      }`}
    >
      <div className="px-3 py-1.5 border-b border-ds-border flex items-center justify-between shrink-0">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Whale Radar
        </span>
        <span className="text-[10px] font-mono text-ds-blue tabular-nums">
          {whales.length + smart.length}
        </span>
      </div>
      <div
        className={`flex-1 overflow-y-auto terminal-scroll p-1.5 space-y-1.5 ${
          compact && !fillHeight ? 'max-h-56' : 'min-h-0'
        }`}
      >
        {whales.length === 0 && smart.length === 0 ? (
          <p className="text-[10px] text-ds-text-muted text-center py-6 leading-relaxed">
            Scanning chain…
            <span className="block font-mono text-[9px] mt-1 opacity-70">:8090 intelligence</span>
          </p>
        ) : (
          <>
            {whales.map((w) => (
              <article
                key={w.alert_id}
                className="rounded-terminal border border-ds-blue/25 bg-ds-blue/5 px-2 py-1.5"
              >
                <div className="flex justify-between items-start gap-1">
                  <div className="flex items-center gap-1 flex-wrap min-w-0">
                    <span className="text-[9px] font-bold text-ds-blue uppercase tracking-wide">
                      {w.dex}
                    </span>
                    <span
                      className="text-[8px] font-mono px-1 py-px border rounded-terminal"
                      style={{ color: tierColor(w.tier), borderColor: `${tierColor(w.tier)}40` }}
                    >
                      {tierLabel(w.tier)}
                    </span>
                  </div>
                  <span className="text-[9px] font-mono text-ds-text-muted shrink-0">
                    {(w.confidence * 100).toFixed(0)}%
                  </span>
                </div>
                <p className="text-[13px] font-semibold font-mono text-ds-text-primary mt-0.5 leading-none">
                  {w.amount_sol.toFixed(2)} SOL
                  <span className="text-[10px] text-ds-text-muted ml-1.5 font-normal">
                    ${w.notional_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                  </span>
                </p>
                <p className="text-[9px] font-mono text-ds-text-muted mt-0.5 truncate">
                  {w.token_symbol} · {shortPool(w.wallet, 4, 4)}
                </p>
              </article>
            ))}
            {smart.map((s) => (
              <article
                key={s.alert_id}
                className="rounded-terminal border border-purple-500/25 bg-purple-500/5 px-2 py-1.5"
              >
                <div className="flex justify-between">
                  <span className="text-[9px] font-bold text-purple-400 uppercase">Smart $</span>
                  <span className="text-[9px] font-mono text-ds-text-muted">
                    {(s.strength * 100).toFixed(0)}%
                  </span>
                </div>
                <p className="text-[11px] font-mono text-ds-text-primary mt-0.5">
                  {s.amount_sol.toFixed(2)} SOL · {s.token_symbol}
                </p>
              </article>
            ))}
          </>
        )}
      </div>
    </section>
  );
}
