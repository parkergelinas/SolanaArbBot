'use client';

import { useEffect } from 'react';

import { scanCrossDexArb } from '@/lib/arb/crossDexScanner';
import LiveHubBootstrap from '@/components/terminal/LiveHubBootstrap';
import StreamBootstrap from '@/components/terminal/StreamBootstrap';
import { WATCHLIST } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useWatchlistStore } from '@/stores/watchlistStore';

const ARB_POLL_MS = 12_000;

function CrossDexArbBootstrap() {
  const mergeArbOpportunities = useMarketStore((s) => s.mergeArbOpportunities);
  const entries = useWatchlistStore((s) => s.entries);

  useEffect(() => {
    let cancelled = false;

    const mints = () => {
      const fromWatch = entries.map((e) => ({ mint: e.mint, symbol: e.symbol }));
      const base = WATCHLIST.map((t) => ({ mint: t.mint, symbol: t.symbol }));
      const seen = new Set<string>();
      const merged: { mint: string; symbol: string }[] = [];
      for (const m of [...fromWatch, ...base]) {
        if (seen.has(m.mint)) continue;
        seen.add(m.mint);
        merged.push(m);
      }
      return merged.slice(0, 12);
    };

    const poll = async () => {
      try {
        const opps = await scanCrossDexArb(mints());
        if (!cancelled && opps.length > 0) {
          mergeArbOpportunities(opps);
        }
      } catch {
        /* DexScreener rate limit / offline */
      }
    };

    void poll();
    const id = setInterval(() => void poll(), ARB_POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [entries, mergeArbOpportunities]);

  return null;
}

/** Connects stream-api, control-api hub, and DexScreener arb on every page. */
export default function GlobalFeedsBootstrap() {
  return (
    <>
      <StreamBootstrap />
      <LiveHubBootstrap />
      <CrossDexArbBootstrap />
    </>
  );
}
