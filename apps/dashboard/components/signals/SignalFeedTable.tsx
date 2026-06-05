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
    <div className="flex items-center gap-2 justify-end">
      <div className="w-16 h-1 bg-platform-border rounded-full overflow-hidden hidden sm:block">
        <div
          className="h-full rounded-full"
          style={{ width: `${Math.round(value * 100)}%`, backgroundColor: color }}
        />
      </div>
      <span className="mono text-xs text-slate-300 tabular-nums w-10 text-right">
        {formatPct(value)}
      </span>
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

  if (loading && signals.length === 0) {
    return (
      <div className="surface-card p-12 text-center">
        <div className="inline-block w-5 h-5 border-2 border-platform-accent/30 border-t-platform-accent rounded-full animate-spin mb-3" />
        <p className="text-sm text-platform-muted">Loading signal history…</p>
      </div>
    );
  }

  if (signals.length === 0) {
    return (
      <div className="surface-card p-12 text-center">
        <p className="text-sm text-slate-300 font-medium">No signals match your filters</p>
        <p className="text-xs text-platform-muted mt-1">
          Start the engine or adjust filters to see the live feed.
        </p>
      </div>
    );
  }

  return (
    <div className="surface-card overflow-hidden">
      <div className="overflow-x-auto">
        <table className="data-table w-full text-sm">
          <thead className="bg-platform-surface border-b border-platform-border">
            <tr>
              <th className="text-left px-4 py-3 w-24">Time</th>
              <th className="text-left px-4 py-3 w-28">Type</th>
              <th className="text-left px-4 py-3 w-20">Side</th>
              <th className="text-left px-4 py-3">Pool</th>
              <th className="text-right px-4 py-3 w-28">Strength</th>
              <th className="text-right px-4 py-3 w-28">Confidence</th>
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
                  className={`cursor-pointer ${selected ? 'selected' : ''}`}
                >
                  <td className="px-4 py-3 whitespace-nowrap">
                    <div className="flex items-center gap-2">
                      {isLive && (
                        <span className="w-1.5 h-1.5 rounded-full bg-platform-accent live-pulse shrink-0" />
                      )}
                      <span className="mono text-xs text-platform-muted tabular-nums">
                        {formatRelativeTime(s.timestamp_micros, now)}
                      </span>
                    </div>
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className="inline-flex px-2 py-0.5 rounded-md text-[11px] font-medium"
                      style={{ color: typeMeta.color, backgroundColor: typeMeta.bg }}
                    >
                      {typeMeta.label}
                    </span>
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className="inline-flex px-2 py-0.5 rounded-md text-[11px] font-medium"
                      style={{ color: dirMeta.color, backgroundColor: dirMeta.bg }}
                    >
                      {s.direction}
                    </span>
                  </td>
                  <td className="px-4 py-3">
                    <span className="mono text-xs text-slate-300">
                      {shortPool(s.pool_address)}
                    </span>
                  </td>
                  <td className="px-4 py-3">
                    <StrengthCell value={s.strength} color={typeMeta.color} />
                  </td>
                  <td className="px-4 py-3">
                    <StrengthCell value={s.confidence} color="#8b95a8" />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
