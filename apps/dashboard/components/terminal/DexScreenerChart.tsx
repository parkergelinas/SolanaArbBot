'use client';

import { useMemo } from 'react';

import { dexScreenerEmbedUrl, dexScreenerPairPageUrl } from '@/lib/dexscreener/client';
import { useDexScreenerContext } from '@/components/terminal/DexScreenerProvider';
import { tokenSymbol } from '@/lib/terminal/tokens';
import { useUiStore } from '@/stores/uiStore';

export default function DexScreenerChart({ compact = false }: { compact?: boolean }) {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const symbol = selectedMint ? tokenSymbol(selectedMint) : '—';
  const { snapshot, loading, error } = useDexScreenerContext();

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
    <section className="flex flex-col h-full min-h-0 bg-ds-base">
      <div
        className={`flex items-center justify-between px-3 h-8 border-b border-ds-border shrink-0 gap-2 ${
          compact ? 'hidden' : ''
        }`}
      >
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[11px] font-medium uppercase tracking-[0.1em] text-ds-text-secondary shrink-0">
            Chart
          </span>
          {snapshot && (
            <span className="text-[10px] font-mono text-ds-text-muted truncate">
              {snapshot.baseSymbol}/{snapshot.quoteSymbol} · {snapshot.dexId}
            </span>
          )}
        </div>
        <a
          href={externalUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] text-ds-blue hover:underline shrink-0"
        >
          DexScreener ↗
        </a>
      </div>

      <div className="flex-1 min-h-[200px] relative bg-ds-base">
        {loading && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 text-ds-text-muted z-10 bg-ds-base/90">
            <span className="text-[11px] font-mono">Loading {symbol} chart…</span>
          </div>
        )}

        {!loading && error && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-center px-6 z-10">
            <p className="text-[11px] text-ds-text-muted">{error}</p>
            <a
              href={externalUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[11px] px-3 py-1.5 border border-ds-border text-ds-blue hover:bg-ds-elevated rounded-terminal"
            >
              Search {symbol}
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
