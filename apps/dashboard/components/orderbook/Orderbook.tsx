'use client';

import { formatSpread, formatPrice } from '@/lib/formatters';
import OrderbookRow from './OrderbookRow';
import { useOrderbookData } from './useOrderbookData';

export default function Orderbook() {
  const { symbol, midPrice, spread, spreadPct, bids, asks, hasLiveDepth, offline } =
    useOrderbookData();
  const midRef = midPrice || asks[0]?.price || bids[0]?.price || 1;
  const spreadFmt = formatSpread(spread, midRef);
  const spreadWide = spreadPct > 0.1;
  const empty = !hasLiveDepth;

  return (
    <section className="flex flex-col h-full min-h-0 bg-ds-surface border-r border-ds-border overflow-hidden">
      <div className="terminal-panel-header justify-between shrink-0">
        <span>Orderbook</span>
        <span className="font-mono normal-case tracking-normal text-ds-text-primary">
          {symbol}/USDC{' '}
          {midPrice > 0 && (
            <span className="text-ds-text-secondary">${formatPrice(midPrice)}</span>
          )}
        </span>
      </div>

      <div className="grid grid-cols-[1fr_1fr_1fr] gap-0 px-2 py-1 text-[11px] uppercase tracking-[0.08em] text-ds-text-secondary border-b border-ds-border shrink-0">
        <span className="text-right">Size</span>
        <span className="text-center">Price</span>
        <span className="text-right">Depth</span>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {empty ? (
          <div className="flex flex-col items-center justify-center h-full px-4 py-8 text-center">
            <p className="text-[11px] text-ds-text-muted font-ui leading-relaxed">
              {offline
                ? 'No live depth — start stream-api on port 8080 or check NEXT_PUBLIC_STREAM_URL.'
                : 'Awaiting cross-DEX quotes from the market stream.'}
            </p>
            {midPrice > 0 && (
              <p className="mt-2 text-[12px] font-mono text-ds-text-secondary">
                Mid ${formatPrice(midPrice)} · DexScreener
              </p>
            )}
          </div>
        ) : (
          <>
            {asks
              .slice()
              .reverse()
              .map((level, i) => (
                <OrderbookRow key={`a-${i}`} level={level} side="ask" />
              ))}

            <div className="flex items-center justify-center gap-2 h-[22px] border-y border-ds-border text-[11px] font-mono text-ds-text-secondary">
              <span>SPREAD</span>
              <span>{spread > 0 ? spreadFmt.abs : '—'}</span>
              <span className={spreadWide ? 'text-ds-amber' : ''}>
                {spreadPct > 0 ? `(${spreadFmt.pct})` : '(—)'}
              </span>
            </div>

            {bids.map((level, i) => (
              <OrderbookRow key={`b-${i}`} level={level} side="bid" />
            ))}
          </>
        )}
      </div>
    </section>
  );
}
