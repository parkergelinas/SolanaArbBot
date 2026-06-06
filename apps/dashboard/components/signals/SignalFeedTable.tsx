'use client';

import { useEffect, useState } from 'react';

import {
  DIRECTION_META,
  formatRelativeTime,
  shortPool,
  SIGNAL_TYPE_META,
} from '@/lib/signals';
import type { SignalEvent } from '@/lib/types';
import { formatPct } from '@/lib/types';

interface SignalFeedTableProps {
  signals: SignalEvent[];
  liveIds: Set<number>;
  selectedId: number | null;
  onSelect: (id: number) => void;
  loading: boolean;
}

function StrengthCell({ value, color }: { value: number; color: string }) {
  return (
    <div className="flex items-center gap-1.5 justify-end">
      <div className="w-12 h-0.5 bg-ds-border rounded-full overflow-hidden hidden sm:block">
        <div
          className="h-full rounded-full"
          style={{ width: `${Math.round(value * 100)}%`, backgroundColor: color }}
        />
      </div>
      <span className="font-mono text-[10px] text-ds-text-secondary tabular-nums w-9 text-right">
        {formatPct(value)}
      </span>
    </div>
  );
}

function EmptyState({ title, detail }: { title: string; detail?: string }) {
  return (
    <div className="flex flex-col items-center justify-center flex-1 min-h-[200px] px-6 py-10 text-center">
      <span className="text-2xl mb-2 opacity-40">⚡</span>
      <p className="text-[12px] font-medium text-ds-text-secondary">{title}</p>
      {detail && <p className="text-[10px] text-ds-text-muted mt-1 max-w-xs">{detail}</p>}
    </div>
  );
}

export default function SignalFeedTable({
  signals,
  liveIds,
  selectedId,
  onSelect,
  loading,
}: SignalFeedTableProps) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <div className="flex flex-col h-full min-h-0 bg-ds-surface">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-ds-border shrink-0">
        <span className="text-[10px] uppercase tracking-[0.14em] text-ds-text-muted font-semibold">
          Signal Feed
        </span>
        <span className="text-[10px] font-mono text-ds-text-muted tabular-nums">
          {signals.length} signals
        </span>
      </div>

      {loading && signals.length === 0 ? (
        <div className="flex flex-col items-center justify-center flex-1 min-h-[200px] gap-2">
          <div className="w-4 h-4 border-2 border-ds-blue/30 border-t-ds-blue rounded-full animate-spin" />
          <p className="text-[11px] text-ds-text-muted">Loading history…</p>
        </div>
      ) : signals.length === 0 ? (
        <EmptyState
          title="No signals match filters"
          detail="Start the engine or widen filters to populate the feed."
        />
      ) : (
        <div className="flex-1 min-h-0 overflow-auto terminal-scroll mobile-table-scroll">
          <table className="w-full text-[11px] min-w-[32rem]">
            <thead className="sticky top-0 z-10 bg-ds-elevated border-b border-ds-border">
              <tr>
                {['Time', 'Type', 'Side', 'Pool', 'Str', 'Conf'].map((h, i) => (
                  <th
                    key={h}
                    className={`px-2 py-1.5 text-[9px] uppercase tracking-[0.12em] font-medium text-ds-text-muted ${
                      i >= 4 ? 'text-right' : 'text-left'
                    } ${i === 0 ? 'w-[4.5rem]' : i === 1 ? 'w-[5.5rem]' : i === 2 ? 'w-[4rem]' : ''}`}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {signals.map((s) => {
                const typeMeta = SIGNAL_TYPE_META[s.signal_type];
                const dirMeta = DIRECTION_META[s.direction];
                const isLive = liveIds.has(s.signal_id);
                const selected = selectedId === s.signal_id;

                return (
                  <tr
                    key={s.signal_id}
                    onClick={() => onSelect(s.signal_id)}
                    className={`cursor-pointer border-b border-ds-border/50 transition-colors ${
                      selected
                        ? 'bg-ds-blue/8 hover:bg-ds-blue/10'
                        : 'hover:bg-ds-elevated/50'
                    }`}
                  >
                    <td className="px-2 py-1.5 whitespace-nowrap">
                      <div className="flex items-center gap-1.5">
                        {isLive && (
                          <span className="w-1 h-1 rounded-full bg-ds-green live-pulse shrink-0" />
                        )}
                        <span className="font-mono text-[10px] text-ds-text-muted tabular-nums">
                          {formatRelativeTime(s.timestamp_micros, now)}
                        </span>
                      </div>
                    </td>
                    <td className="px-2 py-1.5">
                      <span
                        className="inline-flex px-1.5 py-px rounded-terminal text-[9px] font-medium uppercase"
                        style={{ color: typeMeta.color, backgroundColor: typeMeta.bg }}
                      >
                        {typeMeta.label}
                      </span>
                    </td>
                    <td className="px-2 py-1.5">
                      <span
                        className="inline-flex px-1.5 py-px rounded-terminal text-[9px] font-medium"
                        style={{ color: dirMeta.color, backgroundColor: dirMeta.bg }}
                      >
                        {s.direction}
                      </span>
                    </td>
                    <td className="px-2 py-1.5">
                      <span className="font-mono text-[10px] text-ds-text-secondary truncate block max-w-[10rem]">
                        {shortPool(s.pool_address)}
                      </span>
                    </td>
                    <td className="px-2 py-1.5">
                      <StrengthCell value={s.strength} color={typeMeta.color} />
                    </td>
                    <td className="px-2 py-1.5">
                      <StrengthCell value={s.confidence} color="var(--text-secondary)" />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
