import type { DexPairSnapshot } from '@/lib/dexscreener/types';
import type { ConnectionMode } from '@/stores/streamStore';
import type { TokenRow } from '@/stores/marketStore';
import type { DexAnchor, ResolvedPrice } from './types';

export interface PriceInputs {
  mint: string;
  connectionMode: ConnectionMode;
  dex?: DexPairSnapshot | null;
  anchor?: DexAnchor | null;
  streamToken?: TokenRow;
}

const STALE_MS = 120_000;
/** Reject stream quotes that diverge >25% from Dex anchor (stale sim contamination). */
const STREAM_ANCHOR_MAX_DRIFT = 0.25;

function streamMatchesAnchor(streamPx: number, anchorPx: number): boolean {
  if (!anchorPx || anchorPx <= 0) return true;
  return Math.abs(streamPx - anchorPx) / anchorPx <= STREAM_ANCHOR_MAX_DRIFT;
}

/**
 * Display price priority:
 * 1. Fresh DexScreener quote (poll or selected-pair context)
 * 2. Live stream when consistent with anchor (or no anchor yet)
 * 3. Dex anchor cache (blocks stale sim $145 when stream diverges)
 * 4. Sim stream (last resort, marked stale)
 */
export function resolveTokenPrice(input: PriceInputs): ResolvedPrice {
  const now = Date.now();
  const anchorPx = input.anchor?.priceUsd ?? 0;
  const streamPx = input.streamToken?.price_usd ?? 0;
  const streamLive =
    input.connectionMode === 'live' || input.connectionMode === 'degraded';

  if (input.dex?.priceUsd && input.dex.priceUsd > 0) {
    return {
      priceUsd: input.dex.priceUsd,
      changeH24Pct: input.dex.changeH24Pct ?? 0,
      volumeH24Usd: input.dex.volumeH24Usd ?? 0,
      source: 'dex',
      stale: false,
    };
  }

  if (streamLive && streamPx > 0 && streamMatchesAnchor(streamPx, anchorPx)) {
    return {
      priceUsd: streamPx,
      changeH24Pct: input.streamToken?.changePct ?? 0,
      volumeH24Usd: input.streamToken?.volume ?? 0,
      source: 'stream',
      stale: false,
    };
  }

  if (anchorPx > 0) {
    const stale = now - (input.anchor?.fetchedAt ?? 0) > STALE_MS;
    return {
      priceUsd: anchorPx,
      changeH24Pct: input.anchor?.changeH24Pct ?? 0,
      volumeH24Usd: input.anchor?.volumeH24Usd ?? 0,
      source: 'dex',
      stale,
    };
  }

  if (streamPx > 0) {
    return {
      priceUsd: streamPx,
      changeH24Pct: input.streamToken?.changePct ?? 0,
      volumeH24Usd: input.streamToken?.volume ?? 0,
      source: 'sim',
      stale: true,
    };
  }

  return {
    priceUsd: 0,
    changeH24Pct: 0,
    volumeH24Usd: 0,
    source: 'none',
    stale: true,
  };
}

/** Anchor for swap sanitization — never hardcoded $145. */
export function anchorPriceUsd(
  mint: string,
  anchors: Record<string, DexAnchor>,
  livePrice?: number,
): number {
  if (livePrice !== undefined && Number.isFinite(livePrice) && livePrice > 0) {
    return livePrice;
  }
  const anchor = anchors[mint];
  if (anchor?.priceUsd && anchor.priceUsd > 0) return anchor.priceUsd;
  return 0;
}
