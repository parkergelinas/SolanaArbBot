'use client';

import { useCallback, useMemo, useState } from 'react';

import SignalDetailPanel from '@/components/signals/SignalDetailPanel';
import SignalFeedTable from '@/components/signals/SignalFeedTable';
import SignalFilters from '@/components/signals/SignalFilters';
import SignalStatsBar from '@/components/signals/SignalStatsBar';
import { useWsContext } from '@/components/WebSocketProvider';
import { api } from '@/lib/api';
import { useFetch, useStreamSignals } from '@/lib/hooks';
import {
  filterSignals,
  mergeSignals,
  sortSignals,
  type SignalFilterDirection,
  type SignalFilterType,
  type SignalSortKey,
} from '@/lib/signals';
import type { SignalEvent } from '@/lib/types';

export default function SignalsPage() {
  const { connected } = useWsContext();
  const [typeFilter, setTypeFilter] = useState<SignalFilterType>('All');
  const [dirFilter, setDirFilter] = useState<SignalFilterDirection>('All');
  const [sortKey, setSortKey] = useState<SignalSortKey>('time');
  const [selectedId, setSelectedId] = useState<number | null>(null);

  const fetcher = useCallback(
    () => api.signals({ limit: 200, signal_type: typeFilter === 'All' ? undefined : typeFilter }),
    [typeFilter],
  );
  const { data: historical, loading, error } = useFetch(fetcher, 15_000);
  const liveSignals = useStreamSignals<SignalEvent>(500);

  const organized = useMemo(
    () => mergeSignals(liveSignals, historical),
    [liveSignals, historical],
  );

  const filtered = useMemo(() => {
    const base = filterSignals(organized.signals, typeFilter, dirFilter);
    return sortSignals(base, sortKey);
  }, [organized.signals, typeFilter, dirFilter, sortKey]);

  const selected = useMemo(
    () => filtered.find((s) => s.signal_id === selectedId) ?? null,
    [filtered, selectedId],
  );

  return (
    <div className="flex flex-col gap-5 max-w-7xl pb-8">
      <header className="flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100 tracking-tight">Signals</h1>
          <p className="text-platform-muted text-sm mt-0.5">
            Real-time market intelligence — whale flow, smart money, momentum
          </p>
        </div>
        {error && (
          <p className="text-xs text-amber-400 bg-amber-500/10 border border-amber-500/20 rounded-lg px-3 py-1.5 max-w-xl">
            Historical signals unavailable ({error}). Live WebSocket feed may still work — ensure
            control-api is running locally or set <code className="text-amber-200">CONTROL_API_URL</code> on
            Vercel.
          </p>
        )}
      </header>

      <SignalStatsBar stats={organized.stats} connected={connected} />

      <SignalFilters
        type={typeFilter}
        direction={dirFilter}
        sort={sortKey}
        count={filtered.length}
        onType={setTypeFilter}
        onDirection={setDirFilter}
        onSort={setSortKey}
      />

      <div className="flex flex-col lg:flex-row gap-4 items-start">
        <div className="flex-1 min-w-0 w-full">
          <SignalFeedTable
            signals={filtered}
            liveIds={organized.liveIds}
            selectedId={selectedId}
            onSelect={setSelectedId}
            loading={loading}
          />
        </div>
        <SignalDetailPanel
          signal={selected}
          isLive={selected ? organized.liveIds.has(selected.signal_id) : false}
        />
      </div>
    </div>
  );
}
