import type { MarketState, TokenRow } from './marketStore';

/** Stable empty — avoids new object refs when mint has no row yet. */
const EMPTY_TOKEN: TokenRow = {
  mint: '',
  price_usd: 0,
  volume: 0,
  buyFlow: 0,
  sellFlow: 0,
  flowImbalance: 0,
  changePct: 0,
  slot: 0,
  timestamp_ms: 0,
};

export function selectTokenRow(mint: string | null) {
  return (s: MarketState): TokenRow | undefined =>
    mint ? s.tokens[mint] : undefined;
}

export function selectSwapsTail(max: number) {
  return (s: MarketState) => {
    const tail = s.swaps.length <= max ? s.swaps : s.swaps.slice(-max);
    return tail;
  };
}

/** Minimal revision tuple — avoids re-rendering the swap feed on unrelated store updates. */
export function selectSwapFeedRevision(s: MarketState) {
  const n = s.swaps.length;
  return {
    count: n,
    lastSig: n > 0 ? s.swaps[n - 1]!.signature : '',
    lastSeq: s.lastSeq,
  };
}

export function selectSwapPrices(s: MarketState) {
  return s.prices;
}

export function selectSignals(s: MarketState) {
  return s.signals;
}

export { EMPTY_TOKEN };
