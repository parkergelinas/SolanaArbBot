'use client';

import { useMemo } from 'react';

import { dexSnapshotFromAnchor } from '@/lib/pricing/dexSnapshotFromAnchor';
import { resolveTokenPrice } from '@/lib/pricing/resolve';
import type { DexAnchor } from '@/lib/pricing/types';
import type { ResolvedPrice } from '@/lib/pricing/types';
import type { DexPairSnapshot } from '@/lib/dexscreener/types';
import { useDexScreenerWatchlist } from '@/lib/hooks/useDexScreenerWatchlist';
import { useDexScreenerContext } from '@/components/terminal/DexScreenerProvider';
import { useMarketStore } from '@/stores/marketStore';
import { usePriceAnchorStore } from '@/stores/priceAnchorStore';
import { useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';
import { useWatchlistStore } from '@/stores/watchlistStore';

function dexForMint(
  mint: string,
  snapshots: Record<string, DexPairSnapshot | null | undefined>,
  anchor?: DexAnchor,
): DexPairSnapshot | undefined {
  const snap = snapshots[mint];
  if (snap?.priceUsd && snap.priceUsd > 0) return snap;
  if (anchor?.priceUsd && anchor.priceUsd > 0) {
    return dexSnapshotFromAnchor(anchor, mint);
  }
  return undefined;
}

export function useResolvedPrice(mint: string | null): ResolvedPrice {
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const streamToken = useMarketStore((s) => (mint ? s.tokens[mint] : undefined));
  const anchor = usePriceAnchorStore((s) => (mint ? s.anchors[mint] : undefined));
  const { snapshots } = useDexScreenerWatchlist();

  return useMemo(
    () =>
      resolveTokenPrice({
        mint: mint ?? '',
        connectionMode,
        dex: mint ? dexForMint(mint, snapshots, anchor) : undefined,
        anchor,
        streamToken,
      }),
    [mint, connectionMode, snapshots, anchor, streamToken],
  );
}

/** Selected pair with DexScreener context snapshot (chart pair). */
export function useSelectedPairPrice(): ResolvedPrice {
  const selectedMint = useUiStore((s) => s.selectedMint);
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const streamToken = useMarketStore((s) =>
    selectedMint ? s.tokens[selectedMint] : undefined,
  );
  const anchor = usePriceAnchorStore((s) =>
    selectedMint ? s.anchors[selectedMint] : undefined,
  );
  const { snapshot: dexContext } = useDexScreenerContext();
  const { snapshots } = useDexScreenerWatchlist();

  return useMemo(() => {
    const dex =
      dexContext ??
      (selectedMint ? dexForMint(selectedMint, snapshots, anchor) : undefined);
    return resolveTokenPrice({
      mint: selectedMint ?? '',
      connectionMode,
      dex,
      anchor,
      streamToken,
    });
  }, [selectedMint, connectionMode, dexContext, snapshots, anchor, streamToken]);
}

export function useResolvedPrices(mints: string[]): Record<string, ResolvedPrice> {
  const { snapshots } = useDexScreenerWatchlist();
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const tokens = useMarketStore((s) => s.tokens);
  const anchors = usePriceAnchorStore((s) => s.anchors);
  const mintKey = mints.join('\0');

  return useMemo(() => {
    const out: Record<string, ResolvedPrice> = {};
    for (const mint of mints) {
      out[mint] = resolveTokenPrice({
        mint,
        connectionMode,
        dex: dexForMint(mint, snapshots, anchors[mint]),
        anchor: anchors[mint],
        streamToken: tokens[mint],
      });
    }
    return out;
  }, [mintKey, mints, snapshots, connectionMode, tokens, anchors]);
}

export function useWatchlistResolvedPrices(): Record<string, ResolvedPrice> {
  const entries = useWatchlistStore((s) => s.entries);
  const mints = useMemo(() => entries.map((w) => w.mint), [entries]);
  return useResolvedPrices(mints);
}
