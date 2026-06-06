'use client';

import { useEffect, useRef } from 'react';

import { SOL_MINT } from '@/lib/terminal/tokens';
import { selectSwapFeedRevision } from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';
import { useNetworkStore } from '@/stores/networkStore';
import { usePaperStore } from '@/stores/paperStore';
import { useShallow } from 'zustand/react/shallow';

const AUTO_TRADE_COOLDOWN_MS = 8_000;
const AUTO_TRADE_SOL = 0.25;

/**
 * Optional auto paper-trading: mirrors high-confidence stream swaps on the selected token.
 */
export default function PaperBootstrap() {
  const revision = useMarketStore(useShallow(selectSwapFeedRevision));
  const cluster = useNetworkStore((s) => s.cluster);
  const enabled = usePaperStore((s) => s.enabled);
  const autoTrade = usePaperStore((s) => s.autoTrade);
  const quickTrade = usePaperStore((s) => s.quickTrade);
  const lastTradeAt = useRef(0);

  useEffect(() => {
    if (!enabled || !autoTrade || revision.count === 0) return;

    const now = Date.now();
    if (now - lastTradeAt.current < AUTO_TRADE_COOLDOWN_MS) return;

    const swaps = useMarketStore.getState().swaps;
    const latest = swaps[swaps.length - 1];
    if (!latest) return;

    const prices: Record<string, number> = {};
    const state = useMarketStore.getState();
    for (const [mint, p] of Object.entries(state.prices)) {
      prices[mint] = p.price_usd;
    }
    for (const [mint, t] of Object.entries(state.tokens)) {
      if (!prices[mint]) prices[mint] = t.price_usd;
    }

    const isBuy = latest.token_in === SOL_MINT || latest.token_in.startsWith('So1111');
    const altMint = isBuy ? latest.token_out : latest.token_in;
    if (altMint.startsWith('So1111')) return;

    const ok = quickTrade(altMint, isBuy ? 'buy' : 'sell', AUTO_TRADE_SOL, prices);
    if (ok) lastTradeAt.current = now;
  }, [revision.count, revision.lastSig, enabled, autoTrade, quickTrade, cluster]);

  return null;
}
