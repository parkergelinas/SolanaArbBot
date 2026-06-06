'use client';

import { useEffect } from 'react';

import { setDemoPriceAnchors } from '@/lib/terminal/demoEngine';
import { SCHEMA_VERSION, type TokenPrice, type WSMessage } from '@/lib/stream/types';
import { useMarketStore } from '@/stores/marketStore';
import { usePriceAnchorStore } from '@/stores/priceAnchorStore';
import { useWatchlistStore } from '@/stores/watchlistStore';

/**
 * Fetches live DexScreener USD anchors before demo/sim seeds wrong prices.
 * Injects authoritative token_price messages into the market store.
 */
export default function PriceBootstrap() {
  const bootstrap = usePriceAnchorStore((s) => s.bootstrap);
  const anchors = usePriceAnchorStore((s) => s.anchors);
  const ready = usePriceAnchorStore((s) => s.ready);
  const watchlistHydrated = useWatchlistStore((s) => s.hydrated);
  const entryCount = useWatchlistStore((s) => s.entries.length);
  const applyMessages = useMarketStore((s) => s.applyMessages);

  useEffect(() => {
    if (watchlistHydrated) bootstrap();
  }, [bootstrap, watchlistHydrated, entryCount]);

  useEffect(() => {
    if (!ready || Object.keys(anchors).length === 0) return;

    const priceMap: Record<string, number> = {};
    for (const [mint, a] of Object.entries(anchors)) {
      priceMap[mint] = a.priceUsd;
    }
    setDemoPriceAnchors(priceMap);

    const now = Date.now();
    const messages: WSMessage[] = Object.entries(anchors).map(([mint, a]) => {
      const payload: TokenPrice = {
        v: SCHEMA_VERSION,
        mint,
        price_usd: a.priceUsd,
        slot: 0,
        timestamp_ms: now,
      };
      return { type: 'token_price' as const, payload };
    });

    applyMessages(messages, { seq: 0, ts_ms: now });
  }, [ready, anchors, applyMessages]);

  return null;
}
