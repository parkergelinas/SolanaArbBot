import { useMemo } from 'react';

import { useSelectedPairPrice } from '@/lib/hooks/useResolvedPrice';
import { tokenMeta } from '@/lib/terminal/tokens';
import type { Dex } from '@/lib/stream/types';
import { useMarketStore } from '@/stores/marketStore';
import { useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

export interface OrderbookLevel {
  price: number;
  size: number;
  cumulative: number;
  depthPct: number;
}

export interface OrderbookData {
  symbol: string;
  midPrice: number;
  spread: number;
  spreadPct: number;
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  hasLiveDepth: boolean;
  offline: boolean;
}

function dexSpread(byDex: Record<Dex, number> | undefined, mid: number): number {
  if (!byDex || mid <= 0) return 0;
  const prices = Object.values(byDex).filter((p) => p > 0);
  if (prices.length < 2) return 0;
  return Math.max(...prices) - Math.min(...prices);
}

/** Build coarse levels from cross-DEX quotes (real stream data, not random sim). */
function levelsFromDexQuotes(
  byDex: Record<Dex, number>,
  mid: number,
): { bids: OrderbookLevel[]; asks: OrderbookLevel[] } {
  const entries = Object.entries(byDex).filter(([, p]) => p > 0) as [Dex, number][];
  if (entries.length === 0 || mid <= 0) {
    return { bids: [], asks: [] };
  }

  const sorted = [...entries].sort((a, b) => a[1] - b[1]);
  const bids: OrderbookLevel[] = sorted
    .filter(([, p]) => p <= mid)
    .map(([, p], i) => ({
      price: p,
      size: 1 + i * 0.5,
      cumulative: 0,
      depthPct: 0,
    }));
  const asks: OrderbookLevel[] = sorted
    .filter(([, p]) => p >= mid)
    .map(([, p], i) => ({
      price: p,
      size: 1 + i * 0.5,
      cumulative: 0,
      depthPct: 0,
    }));

  const withCum = (rows: OrderbookLevel[]) => {
    let cum = 0;
    const maxSz = Math.max(...rows.map((r) => r.size), 1);
    return rows.map((r) => {
      cum += r.size;
      return { ...r, cumulative: cum, depthPct: (r.size / maxSz) * 100 };
    });
  };

  return { bids: withCum(bids), asks: withCum(asks) };
}

export function useOrderbookData(): OrderbookData {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const connected = useStreamStore((s) => s.connected);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const resolved = useSelectedPairPrice();
  const dexPrices = useMarketStore((s) =>
    selectedMint ? s.dexPrices[selectedMint] : undefined,
  );

  return useMemo(() => {
    const meta = selectedMint ? tokenMeta(selectedMint) : undefined;
    const symbol = meta?.symbol ?? 'SOL';
    const mid = resolved.priceUsd;

    const streamLive = connected && connectionMode !== 'sim';
    const spread = dexSpread(dexPrices, mid);
    const spreadPct = mid > 0 && spread > 0 ? (spread / mid) * 100 : 0;

    if (!streamLive || !dexPrices || Object.keys(dexPrices).length === 0) {
      return {
        symbol,
        midPrice: mid,
        spread: 0,
        spreadPct: 0,
        bids: [],
        asks: [],
        hasLiveDepth: false,
        offline: !streamLive,
      };
    }

    const { bids, asks } = levelsFromDexQuotes(dexPrices, mid);

    return {
      symbol,
      midPrice: mid,
      spread,
      spreadPct,
      bids,
      asks,
      hasLiveDepth: bids.length > 0 || asks.length > 0,
      offline: false,
    };
  }, [selectedMint, resolved.priceUsd, dexPrices, connected, connectionMode]);
}
