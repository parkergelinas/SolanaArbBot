'use client';

import { useVirtualizer } from '@tanstack/react-virtual';
import { memo, useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';

import { timeAgo } from '@/lib/format/time';
import { tokenSymbol } from '@/lib/terminal/tokens';
import {
  dexBadge,
  formatSwapAmount,
  swapDirection,
  swapPriceImpactBps,
} from '@/lib/terminal/swaps';
import type { SwapEvent, TokenPrice } from '@/lib/stream/types';
import {
  selectSwapFeedRevision,
  selectSwapPrices,
} from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';
import { type ConnectionMode, useStreamStore } from '@/stores/streamStore';
import { useUiStore } from '@/stores/uiStore';

const VISIBLE_BUFFER = 200;
const ROW_HEIGHT = 22;
const SCROLL_STICKY_PX = 48;

const MODE_BADGE: Record<
  ConnectionMode,
  { label: string; className: string }
> = {
  live: {
    label: 'LIVE',
    className: 'text-terminal-accent border-terminal-accent/40 bg-terminal-accent/8',
  },
  degraded: {
    label: 'LIVE',
    className: 'text-terminal-warn border-terminal-warn/40 bg-terminal-warn/8',
  },
  sim: {
    label: 'SIM',
    className: 'text-terminal-warn border-terminal-warn/40 bg-terminal-warn/8',
  },
};

const DEX_CLASS: Record<string, string> = {
  raydium: 'text-violet-400',
  orca: 'text-cyan-400',
  jupiter: 'text-amber-400',
};

const DIR_CLASS: Record<string, string> = {
  buy: 'text-flow-buy',
  sell: 'text-flow-sell',
  swap: 'text-slate-300',
};

const FLASH_CLASS: Record<string, string> = {
  buy: 'swap-flash-buy',
  sell: 'swap-flash-sell',
  swap: 'swap-flash-neutral',
};

function tailSwaps(swaps: SwapEvent[], max: number): SwapEvent[] {
  if (swaps.length <= max) return swaps;
  return swaps.slice(-max);
}

function newestFirst(swaps: SwapEvent[]): SwapEvent[] {
  const out = new Array(swaps.length);
  for (let i = 0; i < swaps.length; i++) {
    out[i] = swaps[swaps.length - 1 - i]!;
  }
  return out;
}

type SwapRowProps = {
  swap: SwapEvent;
  now: number;
  flash: boolean;
  prices: Record<string, TokenPrice>;
};

const SwapRow = memo(function SwapRow({ swap, now, flash, prices }: SwapRowProps) {
  const dir = swapDirection(swap);
  const impact = swapPriceImpactBps(swap, prices);

  return (
    <div
      className={`grid grid-cols-[3.5rem_2.5rem_1fr_4rem_4rem_3rem] gap-0 px-2 mono text-[10px] border-b border-terminal-border/40 hover:bg-terminal-hover ${
        flash ? FLASH_CLASS[dir] : ''
      }`}
      style={{ height: ROW_HEIGHT }}
    >
      <span
        className="text-terminal-muted truncate leading-[22px]"
        title={new Date(swap.timestamp_ms).toLocaleString()}
      >
        {timeAgo(swap.timestamp_ms, now)}
      </span>
      <span
        className={`truncate leading-[22px] font-semibold ${DEX_CLASS[swap.dex] ?? 'text-slate-400'}`}
        title={swap.dex}
      >
        {dexBadge(swap.dex)}
      </span>
      <span className={`truncate leading-[22px] ${DIR_CLASS[dir]}`}>
        {tokenSymbol(swap.token_in)}→{tokenSymbol(swap.token_out)}
      </span>
      <span className="text-right text-slate-500 truncate leading-[22px]">
        {formatSwapAmount(swap.token_in, swap.amount_in)}
      </span>
      <span className="text-right text-slate-500 truncate leading-[22px]">
        {formatSwapAmount(swap.token_out, swap.amount_out)}
      </span>
      <span
        className={`text-right truncate leading-[22px] ${
          impact === null
            ? 'text-terminal-muted/50'
            : impact >= 0
              ? 'text-flow-buy/80'
              : 'text-flow-sell/80'
        }`}
        title={impact !== null ? `${impact} bps vs ref` : undefined}
      >
        {impact !== null ? `${impact > 0 ? '+' : ''}${impact}` : '—'}
      </span>
    </div>
  );
});

export default function VirtualizedSwapFeed() {
  const revision = useMarketStore(useShallow(selectSwapFeedRevision));
  const prices = useMarketStore(useShallow(selectSwapPrices));
  const connectionMode = useStreamStore((s) => s.connectionMode);
  const connected = useStreamStore((s) => s.connected);
  const paused = useUiStore((s) => s.swapTapePaused);
  const setPaused = useUiStore((s) => s.setSwapTapePaused);

  const [displayRows, setDisplayRows] = useState<SwapEvent[]>([]);
  const [pendingBehind, setPendingBehind] = useState(0);
  const [flashSigs, setFlashSigs] = useState<Set<string>>(() => new Set());
  const [now, setNow] = useState(Date.now);

  const displayRowsRef = useRef<SwapEvent[]>([]);
  const prevDisplaySigsRef = useRef<Set<string>>(new Set());
  const parentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 5_000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    const storeTail = newestFirst(
      tailSwaps(useMarketStore.getState().swaps, VISIBLE_BUFFER),
    );

    if (paused) {
      const frozenSigs = new Set(displayRowsRef.current.map((s) => s.signature));
      const pending = storeTail.filter((s) => !frozenSigs.has(s.signature)).length;
      setPendingBehind(pending);
      return;
    }

    const prevSigs = prevDisplaySigsRef.current;
    const newSigs = storeTail.filter((s) => !prevSigs.has(s.signature)).map((s) => s.signature);

    if (newSigs.length > 0) {
      setFlashSigs(new Set(newSigs));
      const timer = setTimeout(() => setFlashSigs(new Set()), 650);
      prevDisplaySigsRef.current = new Set(storeTail.map((s) => s.signature));
      displayRowsRef.current = storeTail;
      setDisplayRows(storeTail);
      setPendingBehind(0);

      const el = parentRef.current;
      if (el && el.scrollTop <= SCROLL_STICKY_PX) {
        requestAnimationFrame(() => {
          el.scrollTop = 0;
        });
      }

      return () => clearTimeout(timer);
    }

    if (
      storeTail.length !== displayRowsRef.current.length ||
      storeTail.some((s, i) => s.signature !== displayRowsRef.current[i]?.signature)
    ) {
      prevDisplaySigsRef.current = new Set(storeTail.map((s) => s.signature));
      displayRowsRef.current = storeTail;
      setDisplayRows(storeTail);
    }
    setPendingBehind(0);
  }, [revision.count, revision.lastSig, revision.lastSeq, paused]);

  const virtualizer = useVirtualizer({
    count: displayRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
    getItemKey: (index) => displayRows[index]?.signature ?? index,
  });

  const mode = MODE_BADGE[connectionMode];
  const emptyMessage =
    connected && revision.count === 0
      ? 'Syncing live swaps…'
      : connected
        ? 'No swaps yet'
        : 'Start stream-api or enable demo mode';

  return (
    <section className="flex flex-col flex-shrink-0 h-44 min-h-0 border-t border-terminal-border bg-terminal-panel">
      <div className="px-2 py-1 border-b border-terminal-border flex justify-between items-center gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[10px] font-semibold uppercase tracking-widest text-terminal-muted">
            Live Swaps
          </span>
          <span
            className={`text-[8px] mono font-semibold px-1 py-px rounded border uppercase tracking-wider ${mode.className}`}
          >
            {mode.label}
          </span>
          <span className="text-[8px] mono text-terminal-muted/70 tabular-nums">
            {displayRows.length}
          </span>
          {paused && pendingBehind > 0 && (
            <span className="text-[8px] mono text-terminal-warn">+{pendingBehind} queued</span>
          )}
        </div>
        <button
          type="button"
          onClick={() => setPaused(!paused)}
          className="text-[10px] text-terminal-muted hover:text-terminal-live shrink-0"
        >
          {paused ? 'Resume' : 'Pause'}
        </button>
      </div>

      <div className="grid grid-cols-[3.5rem_2.5rem_1fr_4rem_4rem_3rem] gap-0 px-2 py-0.5 text-[9px] uppercase text-terminal-muted border-b border-terminal-border shrink-0">
        <span>Time</span>
        <span>DEX</span>
        <span>Pair</span>
        <span className="text-right">In</span>
        <span className="text-right">Out</span>
        <span className="text-right">Impact</span>
      </div>

      <div ref={parentRef} className="flex-1 min-h-0 overflow-y-auto">
        {displayRows.length === 0 ? (
          <div className="p-4 text-center text-[11px] text-terminal-muted">{emptyMessage}</div>
        ) : (
          <div
            style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}
          >
            {virtualizer.getVirtualItems().map((vRow) => {
              const s = displayRows[vRow.index];
              return (
                <div
                  key={s.signature}
                  className="absolute left-0 w-full"
                  style={{
                    height: ROW_HEIGHT,
                    transform: `translateY(${vRow.start}px)`,
                  }}
                >
                  <SwapRow
                    swap={s}
                    now={now}
                    flash={flashSigs.has(s.signature)}
                    prices={prices}
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
