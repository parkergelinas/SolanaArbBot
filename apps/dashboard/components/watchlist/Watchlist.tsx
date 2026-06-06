'use client';

import { useEffect, useMemo, useState } from 'react';

import { useWatchlistResolvedPrices } from '@/lib/hooks/useResolvedPrice';
import { useUiStore } from '@/stores/uiStore';
import { useWatchlistStore } from '@/stores/watchlistStore';
import WatchlistAddForm from './WatchlistAddForm';
import WatchlistRow from './WatchlistRow';

export interface WatchlistRowData {
  mint: string;
  symbol: string;
  price: number;
  changePct: number;
  volume: number;
  stale: boolean;
  custom: boolean;
}

type SortKey = 'symbol' | 'price' | 'changePct' | 'volume';
type SortDir = 'asc' | 'desc';

function sortIndicator(active: boolean, dir: SortDir): string {
  if (!active) return '';
  return dir === 'asc' ? ' ▲' : ' ▼';
}

export default function Watchlist() {
  const entries = useWatchlistStore((s) => s.entries);
  const setAddOpen = useWatchlistStore((s) => s.setAddOpen);
  const removeToken = useWatchlistStore((s) => s.removeToken);
  const resolved = useWatchlistResolvedPrices();
  const selectedMint = useUiStore((s) => s.selectedMint);
  const setSelectedMint = useUiStore((s) => s.setSelectedMint);

  const [sortKey, setSortKey] = useState<SortKey>('symbol');
  const [sortDir, setSortDir] = useState<SortDir>('asc');

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      e.preventDefault();
      const idx = entries.findIndex((w) => w.mint === selectedMint);
      const base = idx >= 0 ? idx : 0;
      const next =
        e.key === 'ArrowUp'
          ? Math.max(0, base - 1)
          : Math.min(entries.length - 1, base + 1);
      setSelectedMint(entries[next]?.mint ?? null);
    };

    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selectedMint, setSelectedMint, entries]);

  const rows = useMemo(() => {
    const base: WatchlistRowData[] = entries.map((w) => {
      const r = resolved[w.mint];
      return {
        mint: w.mint,
        symbol: w.symbol,
        price: r?.priceUsd ?? 0,
        changePct: r?.changeH24Pct ?? 0,
        volume: r?.volumeH24Usd ?? 0,
        stale: r?.stale ?? true,
        custom: w.custom,
      };
    });

    return [...base].sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (typeof av === 'string' && typeof bv === 'string') {
        return sortDir === 'asc' ? av.localeCompare(bv) : bv.localeCompare(av);
      }
      return sortDir === 'asc'
        ? (av as number) - (bv as number)
        : (bv as number) - (av as number);
    });
  }, [entries, resolved, sortKey, sortDir]);

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('desc');
    }
  };

  const handleRemove = (mint: string) => {
    if (removeToken(mint) && selectedMint === mint) {
      const remaining = entries.filter((e) => e.mint !== mint);
      setSelectedMint(remaining[0]?.mint ?? null);
    }
  };

  return (
    <aside className="flex flex-col min-h-0 shrink-0 border-r border-ds-border bg-ds-surface overflow-hidden">
      <div className="terminal-panel-header justify-between shrink-0">
        <span>Markets</span>
        <button
          type="button"
          className="text-ds-blue normal-case tracking-normal text-[11px] hover:underline"
          onClick={() => setAddOpen(true)}
        >
          + ADD
        </button>
      </div>

      <WatchlistAddForm />

      <div className="grid grid-cols-[3.5rem_1fr_3rem_2.5rem] gap-1 px-2 py-1 text-[11px] uppercase tracking-[0.08em] text-ds-text-secondary border-b border-ds-border shrink-0 font-ui">
        <button type="button" className="text-left" onClick={() => toggleSort('symbol')}>
          Token{sortIndicator(sortKey === 'symbol', sortDir)}
        </button>
        <button type="button" className="text-right" onClick={() => toggleSort('price')}>
          Price{sortIndicator(sortKey === 'price', sortDir)}
        </button>
        <button type="button" className="text-right" onClick={() => toggleSort('changePct')}>
          24h{sortIndicator(sortKey === 'changePct', sortDir)}
        </button>
        <button type="button" className="text-right" onClick={() => toggleSort('volume')}>
          Vol{sortIndicator(sortKey === 'volume', sortDir)}
        </button>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {rows.length === 0 ? (
          <div className="p-3 text-[12px] text-ds-text-muted font-ui text-center">
            No tokens. Press + ADD.
          </div>
        ) : (
          rows.map((row) => (
            <WatchlistRow
              key={row.mint}
              row={row}
              active={row.mint === selectedMint}
              onSelect={() => setSelectedMint(row.mint)}
              onRemove={row.custom ? () => handleRemove(row.mint) : undefined}
            />
          ))
        )}
      </div>
    </aside>
  );
}
