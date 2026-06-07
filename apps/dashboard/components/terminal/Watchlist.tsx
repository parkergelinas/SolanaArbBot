'use client';

import { memo, useEffect, useMemo, useRef, useState } from 'react';

import { useDexScreenerWatchlist } from '@/lib/hooks/useDexScreenerWatchlist';
import { WATCHLIST, tokenMeta } from '@/lib/terminal/tokens';
import { useMarketStore } from '@/stores/marketStore';
import { useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

type SortMode = 'default' | 'change' | 'liquidity';

function fmtPrice(price: number): string {
  if (price >= 100) return price.toFixed(2);
  if (price >= 1) return price.toFixed(4);
  if (price >= 0.01) return price.toFixed(4);
  return price.toFixed(6);
}

function fmtLiq(usd: number): string {
  if (usd >= 1_000_000) return `$${(usd / 1_000_000).toFixed(1)}M`;
  if (usd >= 1_000) return `$${(usd / 1_000).toFixed(0)}K`;
  return `$${usd.toFixed(0)}`;
}

const WatchlistRow = memo(function WatchlistRow({
  mint,
  symbol,
  name,
  price,
  changePct,
  liquidityUsd,
  hasLive,
  active,
  flash,
  onSelect,
}: {
  mint: string;
  symbol: string;
  name: string;
  refPrice: number;
  price: number;
  changePct: number;
  liquidityUsd?: number;
  hasLive: boolean;
  active: boolean;
  flash: 'up' | 'down' | null;
  onSelect: () => void;
}) {
  const up = changePct >= 0;
  return (
    <button
      key={mint}
      type="button"
      onClick={onSelect}
      className={`w-full text-left px-2 py-1.5 border-b border-terminal-border/40 transition-colors ${
        flash === 'up'
          ? 'price-flash-up'
          : flash === 'down'
            ? 'price-flash-down'
            : ''
      } ${
        active
          ? 'bg-terminal-accent/10 border-l-2 border-l-terminal-accent'
          : 'hover:bg-terminal-hover border-l-2 border-l-transparent'
      }`}
    >
      <div className="flex items-center justify-between gap-1">
        <span className={`text-[11px] font-semibold ${active ? 'text-terminal-accent' : 'text-slate-200'}`}>
          {symbol}
        </span>
        {!hasLive && (
          <span className="text-[8px] uppercase text-terminal-muted/60">ref</span>
        )}
      </div>
      <div className="flex items-baseline justify-between mt-0.5">
        <span className="text-[11px] mono tabular-nums text-slate-100">
          ${fmtPrice(price)}
        </span>
        <span className={`text-[10px] mono tabular-nums ${up ? 'text-flow-buy' : 'text-flow-sell'}`}>
          {up ? '+' : ''}
          {changePct.toFixed(2)}%
        </span>
      </div>
      <div className="flex items-center justify-between mt-0.5 gap-1">
        <span className="text-[9px] text-terminal-muted truncate">{name}</span>
        {liquidityUsd != null && liquidityUsd > 0 && (
          <span className="text-[8px] mono text-terminal-muted shrink-0">
            {fmtLiq(liquidityUsd)}
          </span>
        )}
      </div>
    </button>
  );
});

export default function Watchlist() {
  const { snapshots: dexSnapshots } = useDexScreenerWatchlist();
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const selectedMint = useUiStore((s) => s.selectedMint);
  const setSelectedMint = useUiStore((s) => s.setSelectedMint);
  const tokens = useMarketStore((s) => s.tokens);
  const listRef = useRef<HTMLDivElement>(null);

  // ── Search + sort ──────────────────────────────────────────────────────────
  const [search, setSearch] = useState('');
  const [sortMode, setSortMode] = useState<SortMode>('default');

  const SORT_MODES: { key: SortMode; label: string }[] = [
    { key: 'default', label: '—' },
    { key: 'change',  label: '±%' },
    { key: 'liquidity', label: 'Liq' },
  ];

  // ── Price flash ────────────────────────────────────────────────────────────
  const prevPrices = useRef<Record<string, number>>({});
  const flashRef = useRef<Record<string, 'up' | 'down' | null>>({});
  const [, bumpFlash] = useState(0);

  useEffect(() => {
    for (const w of WATCHLIST) {
      const live = tokens[w.mint];
      const dex = dexSnapshots[w.mint];
      const price = live?.price_usd ?? dex?.priceUsd ?? w.refPrice;
      const prev = prevPrices.current[w.mint];
      if (prev !== undefined && price !== prev) {
        flashRef.current[w.mint] = price > prev ? 'up' : 'down';
        bumpFlash((n) => n + 1);
        setTimeout(() => {
          flashRef.current[w.mint] = null;
          bumpFlash((n) => n + 1);
        }, 650);
      }
      prevPrices.current[w.mint] = price;
    }
  }, [tokens, dexSnapshots]);

  // ── Keyboard navigation ────────────────────────────────────────────────────
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      e.preventDefault();
      const idx = WATCHLIST.findIndex((w) => w.mint === selectedMint);
      const base = idx >= 0 ? idx : 0;
      const next =
        e.key === 'ArrowUp'
          ? Math.max(0, base - 1)
          : Math.min(WATCHLIST.length - 1, base + 1);
      setSelectedMint(WATCHLIST[next].mint);
    };

    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selectedMint, setSelectedMint]);

  // ── Build rows ─────────────────────────────────────────────────────────────
  const rows = useMemo(() => {
    const streamLive = connectionMode === 'live' || connectionMode === 'degraded';
    let list = WATCHLIST.map((w) => {
      const live = tokens[w.mint];
      const dex = dexSnapshots[w.mint];
      const price = streamLive
        ? (live?.price_usd ?? dex?.priceUsd ?? w.refPrice)
        : (dex?.priceUsd ?? live?.price_usd ?? w.refPrice);
      const changePct = dex?.changeH24Pct ?? live?.changePct ?? 0;
      return {
        ...w,
        price,
        changePct,
        liquidityUsd: dex?.liquidityUsd,
        hasLive: Boolean(live) || Boolean(dex),
      };
    });

    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(
        (r) =>
          r.symbol.toLowerCase().includes(q) ||
          r.name.toLowerCase().includes(q),
      );
    }

    if (sortMode === 'change') {
      list = [...list].sort((a, b) => b.changePct - a.changePct);
    } else if (sortMode === 'liquidity') {
      list = [...list].sort((a, b) => (b.liquidityUsd ?? 0) - (a.liquidityUsd ?? 0));
    }

    return list;
  }, [tokens, dexSnapshots, connectionMode, search, sortMode]);

  return (
    <aside
      ref={listRef}
      className="terminal-watchlist flex flex-col min-h-0 w-[11.5rem] shrink-0 border-r border-terminal-border bg-terminal-panel"
      tabIndex={0}
    >
      {/* ── Header ──────────────────────────────────────────────────────────── */}
      <div className="px-2 py-1.5 border-b border-terminal-border flex items-center justify-between shrink-0">
        <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
          Watchlist
        </span>
        <span className="text-[9px] text-terminal-accent">{rows.length}/{WATCHLIST.length}</span>
      </div>

      {/* ── Search bar ─────────────────────────────────────────────────────── */}
      <div className="px-1.5 py-1 border-b border-terminal-border shrink-0 flex items-center gap-1">
        <input
          type="text"
          placeholder="search…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 min-w-0 bg-transparent text-[10px] font-mono text-slate-200 placeholder-terminal-muted/50 outline-none"
          spellCheck={false}
        />
        {search && (
          <button
            type="button"
            onClick={() => setSearch('')}
            className="text-[9px] text-terminal-muted/60 hover:text-terminal-muted shrink-0"
          >
            ✕
          </button>
        )}
      </div>

      {/* ── Sort pills ─────────────────────────────────────────────────────── */}
      <div className="terminal-filterbar shrink-0 !border-t-0">
        {SORT_MODES.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => setSortMode(key)}
            className={`filter-pill ${sortMode === key ? 'active' : ''}`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* ── Rows ──────────────────────────────────────────────────────────── */}
      <div className="flex-1 min-h-0 overflow-y-auto terminal-scroll">
        {rows.length === 0 ? (
          <p className="px-2 py-4 text-[10px] text-terminal-muted text-center">
            No matches
          </p>
        ) : (
          rows.map((row) => (
            <WatchlistRow
              key={row.mint}
              mint={row.mint}
              symbol={row.symbol}
              name={row.name}
              refPrice={row.refPrice}
              price={row.price}
              changePct={row.changePct}
              liquidityUsd={row.liquidityUsd}
              hasLive={row.hasLive}
              active={row.mint === selectedMint}
              flash={flashRef.current[row.mint] ?? null}
              onSelect={() => setSelectedMint(row.mint)}
            />
          ))
        )}
      </div>

      {/* ── Footer ──────────────────────────────────────────────────────────── */}
      {selectedMint && tokenMeta(selectedMint) && (
        <div className="px-2 py-1.5 border-t border-terminal-border text-[9px] text-terminal-muted shrink-0">
          {tokenMeta(selectedMint)?.symbol}/USD · ↑↓ navigate
        </div>
      )}
    </aside>
  );
}
