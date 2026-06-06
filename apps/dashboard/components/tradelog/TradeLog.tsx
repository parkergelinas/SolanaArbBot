'use client';

import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useMemo, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';

import type { PaperFill } from '@/lib/solana/paper';
import type { Signal, SwapEvent } from '@/lib/stream/types';
import { selectSwapFeedRevision } from '@/stores/marketSelectors';
import { useMarketStore } from '@/stores/marketStore';
import { usePaperStore } from '@/stores/paperStore';
import { useUiStore } from '@/stores/uiStore';
import TradeLogRow from './TradeLogRow';

export interface TradeLogEntry {
  id: string;
  kind: 'swap' | 'fill' | 'signal';
  timestampMs: number;
  status: 'pending' | 'settled' | 'reverted';
  swap?: SwapEvent;
  fill?: PaperFill;
  signal?: Signal;
}

const ROW_HEIGHT = 22;
const MAX_ENTRIES = 500;
const EMPTY_FILLS: PaperFill[] = [];

function buildEntries(
  swaps: SwapEvent[],
  fills: PaperFill[],
  signals: Signal[],
): TradeLogEntry[] {
  const entries: TradeLogEntry[] = [];

  for (const s of swaps) {
    entries.push({
      id: `swap-${s.signature}`,
      kind: 'swap',
      timestampMs: s.timestamp_ms,
      status: 'settled',
      swap: s,
    });
  }

  for (const f of fills) {
    entries.push({
      id: `fill-${f.id}`,
      kind: 'fill',
      timestampMs: f.timestampMs,
      status: 'settled',
      fill: f,
    });
  }

  for (const sig of signals) {
    entries.push({
      id: `sig-${sig.signal_id}`,
      kind: 'signal',
      timestampMs: sig.timestamp_ms,
      status: 'pending',
      signal: sig,
    });
  }

  entries.sort((a, b) => a.timestampMs - b.timestampMs);
  return entries.slice(-MAX_ENTRIES);
}

export default function TradeLog() {
  const revision = useMarketStore(useShallow(selectSwapFeedRevision));
  const swaps = useMarketStore((s) => s.swaps);
  const signals = useMarketStore((s) => s.signals);
  const fills = usePaperStore((s) => s.portfolio?.fills ?? EMPTY_FILLS);
  const paused = useUiStore((s) => s.swapTapePaused);
  const setPaused = useUiStore((s) => s.setSwapTapePaused);

  const parentRef = useRef<HTMLDivElement>(null);

  const entries = useMemo(
    () => buildEntries(swaps, fills, signals),
    [revision.count, revision.lastSig, fills.length, signals.length, swaps],
  );

  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
    getItemKey: (index) => entries[index]?.id ?? index,
  });

  useEffect(() => {
    if (paused || entries.length === 0) return;
    const el = parentRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [entries.length, revision.lastSig, paused]);

  return (
    <section className="flex flex-col h-[160px] shrink-0 bg-ds-surface border-t border-ds-border overflow-hidden">
      <div className="terminal-panel-header justify-between shrink-0">
        <span>Trade Log</span>
        <span className="font-mono normal-case tracking-normal text-ds-text-secondary">
          {entries.length}
          {paused && ' · paused'}
        </span>
      </div>

      <div
        ref={parentRef}
        className="flex-1 min-h-0 overflow-y-auto terminal-scroll"
      >
        {entries.length === 0 ? (
          <div className="grid grid-cols-[6.5rem_4rem_3rem_5rem_3.5rem_4rem_4.5rem] gap-1 px-2 py-2 text-[11px] font-mono text-ds-text-muted">
            {Array.from({ length: 5 }).map((_, i) => (
              <span key={i} className="col-span-7">
                — — — — — — —
              </span>
            ))}
          </div>
        ) : (
          <div
            style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}
          >
            {virtualizer.getVirtualItems().map((vRow) => {
              const entry = entries[vRow.index]!;
              return (
                <div
                  key={entry.id}
                  className="absolute left-0 w-full"
                  style={{
                    height: ROW_HEIGHT,
                    transform: `translateY(${vRow.start}px)`,
                  }}
                >
                  <TradeLogRow entry={entry} onPause={() => setPaused(true)} />
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
