'use client';

import { useMemo } from 'react';

import { dexScreenerEmbedUrl, dexScreenerPairPageUrl } from '@/lib/dexscreener/client';
import { useDexScreenerPair } from '@/lib/hooks/useDexScreenerPair';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useUiStore } from '@/stores/uiStore';

export default function DexScreenerChart() {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const { snapshot, loading, error } = useDexScreenerPair(selectedMint);

  const embedUrl = useMemo(
    () => (snapshot ? dexScreenerEmbedUrl(snapshot.pairAddress) : null),
    [snapshot],
  );

  const externalUrl = snapshot
    ? snapshot.pairUrl || dexScreenerPairPageUrl(snapshot.pairAddress)
    : selectedMint
      ? `https://dexscreener.com/search?q=${encodeURIComponent(selectedMint)}`
      : 'https://dexscreener.com';

  return (
    <section className="flex-1 flex flex-col min-h-0 bg-terminal-chart">
      <div className="flex items-center justify-between px-2 py-1 border-b border-terminal-border shrink-0 gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted shrink-0">
            Chart
          </span>
          {snapshot && (
            <span className="text-[10px] mono text-slate-400 truncate">
              {snapshot.baseSymbol}/{snapshot.quoteSymbol} · {snapshot.dexId}
            </span>
          )}
        </div>
        <a
          href={externalUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] text-terminal-accent hover:underline shrink-0"
        >
          Open on DexScreener ↗
        </a>
      </div>

      <div className="flex-1 min-h-[280px] relative bg-[#0d1117]">
        {loading && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 text-terminal-muted z-10 bg-terminal-chart/80">
            <div className="w-8 h-8 border-2 border-terminal-accent/30 border-t-terminal-accent rounded-full animate-spin" />
            <span className="text-[11px]">Resolving {symbol} pair on DexScreener…</span>
          </div>
        )}

        {!loading && error && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-center px-6 z-10">
            <p className="text-[12px] text-terminal-muted">{error}</p>
            <a
              href={externalUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[11px] px-3 py-1.5 rounded border border-terminal-border text-terminal-accent hover:bg-terminal-hover"
            >
              Search {symbol} on DexScreener
            </a>
          </div>
        )}

        {embedUrl && (
          <iframe
            key={embedUrl}
            src={embedUrl}
            title={`DexScreener ${symbol} chart`}
            className="absolute inset-0 w-full h-full border-0"
            allow="clipboard-write"
            loading="lazy"
          />
        )}
      </div>
    </section>
  );
}
