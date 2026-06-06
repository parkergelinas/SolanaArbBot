'use client';

import { usePumpFunScanner } from '@/lib/scanners/hooks';
import { shortPool } from '@/lib/signals';

import MethodologyTip from './MethodologyTip';

export default function PumpFunScannerPanel({
  compact = false,
  fillHeight = false,
}: {
  compact?: boolean;
  fillHeight?: boolean;
}) {
  const { data, loading, error } = usePumpFunScanner();

  const hits = data?.hits ?? [];
  const meta = data?.meta;

  return (
    <section
      className={`bg-ds-surface flex flex-col overflow-hidden ${
        fillHeight
          ? 'h-full min-h-0 rounded-none border-0'
          : compact
            ? 'h-64 border border-ds-border rounded-terminal'
            : 'min-h-64 border border-ds-border rounded-terminal'
      }`}
    >
      <div className="px-3 py-1.5 border-b border-ds-border flex items-center justify-between shrink-0 gap-2">
        <div className="min-w-0">
          <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
            Pump.fun Edge
          </span>
          <p className="text-[9px] text-ds-text-muted truncate">stream-api · DexScreener + Helius launches</p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <MethodologyTip
            title="Early momentum"
            body="Polls DexScreener for pumpfun pairs &lt;12h old with 5m volume and buy-count spikes. Scores graduation proximity (~$69k mcap). Helius logsSubscribe on program 6EF8… enables sub-minute Create detection before pump.fun UI trending."
          />
          <span className="text-[10px] font-mono text-ds-green tabular-nums">{hits.length}</span>
        </div>
      </div>

      {meta?.message && (
        <p className="text-[9px] text-ds-text-secondary bg-ds-elevated/40 border-b border-ds-border px-3 py-1">
          {meta.message}
        </p>
      )}

      {error && (
        <p className="text-[9px] text-ds-red px-3 py-1 border-b border-ds-border">{error}</p>
      )}

      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll p-1.5 space-y-1.5">
        {loading && hits.length === 0 ? (
          <p className="text-[10px] text-ds-text-muted text-center py-6">Watching pump.fun curves…</p>
        ) : hits.length === 0 ? (
          <p className="text-[10px] text-ds-text-muted text-center py-6 leading-relaxed">
            No early momentum detected.
            <span className="block font-mono text-[9px] mt-1 opacity-70">DexScreener · 15s poll</span>
          </p>
        ) : (
          hits.map((h) => (
            <article
              key={h.id}
              className="rounded-terminal border border-ds-green/25 bg-ds-green/5 px-2 py-1.5"
            >
              <div className="flex justify-between items-start gap-1">
                <span className="text-[9px] font-bold text-ds-green uppercase tracking-wide">
                  {h.symbol}
                  {h.onChain && (
                    <span className="ml-1 text-[8px] text-ds-green/90 font-mono normal-case">chain</span>
                  )}
                </span>
                <span className="text-[9px] font-mono text-ds-text-muted">{h.momentumScore}/100</span>
              </div>
              <p className="text-[13px] font-semibold font-mono text-ds-text-primary mt-0.5 leading-none">
                {h.buysM5} buys / 5m
                <span className="text-[10px] text-ds-text-muted ml-1.5 font-normal">
                  ${(h.volumeM5Usd / 1000).toFixed(1)}k vol
                </span>
              </p>
              <p className="text-[9px] font-mono text-ds-text-muted mt-0.5 truncate">
                {h.ageMinutes}m old · grad {h.graduationPct.toFixed(0)}% · {shortPool(h.mint, 4, 4)}
              </p>
              <div className="flex gap-2 mt-1">
                <a
                  href={h.pumpUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[9px] text-ds-green hover:underline"
                >
                  pump.fun
                </a>
                <a
                  href={h.dexUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[9px] text-ds-text-secondary hover:underline"
                >
                  DexScreener
                </a>
              </div>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
