import type { DexPairSnapshot } from '@/lib/dexscreener/types';
import type { DexAnchor } from './types';

/** Build a DexPairSnapshot-shaped object from bootstrap anchor for resolve step 1. */
export function dexSnapshotFromAnchor(anchor: DexAnchor, mint: string): DexPairSnapshot {
  return {
    pairAddress: '',
    dexId: 'anchor',
    pairUrl: '',
    baseSymbol: mint.slice(0, 4),
    quoteSymbol: 'USD',
    priceUsd: anchor.priceUsd,
    changeH24Pct: anchor.changeH24Pct,
    volumeH24Usd: anchor.volumeH24Usd,
    liquidityUsd: anchor.liquidityUsd,
  };
}
