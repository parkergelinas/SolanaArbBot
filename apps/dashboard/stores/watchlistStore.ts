import { create } from 'zustand';

import { WATCHLIST } from '@/lib/terminal/tokens';

export interface WatchlistEntry {
  mint: string;
  symbol: string;
  name: string;
  decimals: number;
  /** User-added token — can be removed. Defaults are pinned. */
  custom: boolean;
}

const STORAGE_KEY = 'solarb_watchlist_custom';
const DEFAULT_ENTRIES: WatchlistEntry[] = WATCHLIST.map((w) => ({
  mint: w.mint,
  symbol: w.symbol,
  name: w.name,
  decimals: w.decimals,
  custom: false,
}));

function loadCustom(): WatchlistEntry[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as WatchlistEntry[];
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((e) => e.mint && e.custom);
  } catch {
    return [];
  }
}

function saveCustom(custom: WatchlistEntry[]) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(custom));
  } catch {
    /* ignore quota */
  }
}

function mergeEntries(custom: WatchlistEntry[]): WatchlistEntry[] {
  const byMint = new Map<string, WatchlistEntry>();
  for (const d of DEFAULT_ENTRIES) byMint.set(d.mint, d);
  for (const c of custom) {
    if (!byMint.has(c.mint)) byMint.set(c.mint, c);
  }
  return Array.from(byMint.values());
}

interface WatchlistState {
  entries: WatchlistEntry[];
  hydrated: boolean;
  addOpen: boolean;

  hydrate: () => void;
  setAddOpen: (open: boolean) => void;
  addToken: (entry: { mint: string; symbol: string; name: string; decimals?: number }) => boolean;
  removeToken: (mint: string) => boolean;
  getMints: () => string[];
}

export const useWatchlistStore = create<WatchlistState>((set, get) => ({
  entries: DEFAULT_ENTRIES,
  hydrated: false,
  addOpen: false,

  hydrate: () => {
    const custom = loadCustom();
    set({ entries: mergeEntries(custom), hydrated: true });
  },

  setAddOpen: (open) => set({ addOpen: open }),

  addToken: (entry) => {
    const mint = entry.mint.trim();
    if (!mint || mint.length < 32) return false;
    const { entries } = get();
    if (entries.some((e) => e.mint === mint)) return false;

    const next: WatchlistEntry = {
      mint,
      symbol: entry.symbol || mint.slice(0, 4),
      name: entry.name || 'Custom',
      decimals: entry.decimals ?? 6,
      custom: true,
    };
    const merged = mergeEntries([
      ...entries.filter((e) => e.custom),
      next,
    ]);
    saveCustom(merged.filter((e) => e.custom));
    set({ entries: merged, addOpen: false });
    return true;
  },

  removeToken: (mint) => {
    const { entries } = get();
    const target = entries.find((e) => e.mint === mint);
    if (!target?.custom) return false;

    const custom = entries.filter((e) => e.custom && e.mint !== mint);
    saveCustom(custom);
    set({ entries: mergeEntries(custom) });
    return true;
  },

  getMints: () => get().entries.map((e) => e.mint),
}));
