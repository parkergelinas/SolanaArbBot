import { create } from 'zustand';

import { bootstrapWatchlistPrices } from '@/lib/pricing/bootstrap';
import type { DexAnchor } from '@/lib/pricing/types';
import { useWatchlistStore } from '@/stores/watchlistStore';

interface PriceAnchorState {
  anchors: Record<string, DexAnchor>;
  ready: boolean;
  loading: boolean;
  lastError: string | null;
  lastHits: number;

  bootstrap: () => Promise<void>;
  refreshMint: (mint: string) => Promise<void>;
  getAnchor: (mint: string) => DexAnchor | undefined;
}

export const usePriceAnchorStore = create<PriceAnchorState>((set, get) => ({
  anchors: {},
  ready: false,
  loading: false,
  lastError: null,
  lastHits: 0,

  getAnchor: (mint) => get().anchors[mint],

  bootstrap: async () => {
    if (get().loading) return;
    set({ loading: true, lastError: null });

    const mints = useWatchlistStore.getState().getMints();

    try {
      const { anchors, hits, errors } = await bootstrapWatchlistPrices(mints);
      set({
        anchors,
        ready: hits > 0,
        loading: false,
        lastHits: hits,
        lastError: errors.length > 0 ? errors.join('; ') : null,
      });
    } catch (e) {
      set({
        loading: false,
        ready: false,
        lastError: e instanceof Error ? e.message : 'Price bootstrap failed',
      });
    }
  },

  refreshMint: async (mint) => {
    const { anchors, hits, errors } = await bootstrapWatchlistPrices([mint]);
    const next = { ...get().anchors, ...anchors };
    set({
      anchors: next,
      ready: Object.keys(next).length > 0,
      lastHits: hits,
      lastError: errors[0] ?? null,
    });
  },
}));
