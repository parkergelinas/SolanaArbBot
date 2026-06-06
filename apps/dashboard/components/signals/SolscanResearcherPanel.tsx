'use client';

import { useSolscanResearcher } from '@/lib/scanners/hooks';
import { shortPool } from '@/lib/signals';

import MethodologyTip from './MethodologyTip';

function timeAgo(ms: number): string {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h`;
}

export default function SolscanResearcherPanel({
  compact = false,
  fillHeight = false,
}: {
  compact?: boolean;
  fillHeight?: boolean;
}) {
  const { data, loading, error } = useSolscanResearcher();

  const hits = data?.hits ?? [];
  const meta = data?.meta;

  return (
    <section
      className={`bg-ds-surface flex flex-col min-h-0 overflow-hidden ${
        fillHeight ? 'h-full rounded-none border-0' : 'border border-ds-border rounded-terminal'
      }`}
    >
      <div className="px-3 py-1.5 border-b border-ds-border flex items-center justify-between shrink-0 gap-2">
        <div className="min-w-0">
          <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
            Solscan Researcher
          </span>
          <p className="text-[9px] text-ds-text-muted truncate">Shitcoin whale wallet discovery</p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <MethodologyTip
            title="Detection method"
            body="Scores low-cap tokens (mcap < $750k) with 5m buy volume spikes via DexScreener. Resolves wallet addresses via Solscan Pro or Helius when API keys are set. Without keys, shows inferred momentum only."
          />
          <span className="text-[10px] font-mono text-ds-amber tabular-nums">{hits.length}</span>
        </div>
      </div>

      {meta?.degraded && (
        <p className="text-[9px] text-ds-amber bg-ds-amber/8 border-b border-ds-amber/20 px-3 py-1">
          {meta.message}
        </p>
      )}

      {error && (
        <p className="text-[9px] text-ds-red px-3 py-1 border-b border-ds-border">{error}</p>
      )}

      <div
        className={`flex-1 overflow-y-auto terminal-scroll p-1.5 space-y-1.5 ${
          compact && !fillHeight ? 'max-h-56' : 'min-h-0'
        }`}
      >
        {loading && hits.length === 0 ? (
          <p className="text-[10px] text-ds-text-muted text-center py-6">Scanning shitcoin flows…</p>
        ) : hits.length === 0 ? (
          <p className="text-[10px] text-ds-text-muted text-center py-6 leading-relaxed">
            No qualifying whale buys right now.
            <span className="block font-mono text-[9px] mt-1 opacity-70">DexScreener · 20s poll</span>
          </p>
        ) : (
          hits.map((h) => (
            <article
              key={h.id}
              className="rounded-terminal border border-ds-amber/25 bg-ds-amber/5 px-2 py-1.5"
            >
              <div className="flex justify-between items-start gap-1">
                <span className="text-[9px] font-bold text-ds-amber uppercase tracking-wide">
                  {h.tokenSymbol}
                </span>
                <span className="text-[9px] font-mono text-ds-text-muted">{h.score}/100</span>
              </div>
              <p className="text-[13px] font-semibold font-mono text-ds-text-primary mt-0.5 leading-none">
                ${h.amountUsd.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                <span className="text-[10px] text-ds-text-muted ml-1.5 font-normal">
                  mcap ${(h.marketCapUsd / 1000).toFixed(0)}k
                </span>
              </p>
              <p className="text-[9px] font-mono text-ds-text-muted mt-0.5 truncate">
                {shortPool(h.wallet, 4, 4)} · {h.source} · {timeAgo(h.detectedAtMs)}
              </p>
              <div className="flex gap-2 mt-1">
                <a
                  href={h.solscanUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[9px] text-ds-blue hover:underline"
                >
                  Solscan
                </a>
                <a
                  href={h.tokenUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[9px] text-ds-text-secondary hover:underline"
                >
                  Token
                </a>
              </div>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
