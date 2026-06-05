'use client';

import { useCallback, useMemo, useState } from 'react';

import TxTape from '@/components/intelligence/TxTape';
import WalletRail from '@/components/intelligence/WalletRail';
import WhaleFeed from '@/components/intelligence/WhaleFeed';
import SignalDetailPanel from '@/components/signals/SignalDetailPanel';
import SignalFeedTable from '@/components/signals/SignalFeedTable';
import SignalFilters from '@/components/signals/SignalFilters';
import SignalStatsBar from '@/components/signals/SignalStatsBar';
import { useWsContext } from '@/components/WebSocketProvider';
import { api } from '@/lib/api';
import { useFetch, useStreamSignals } from '@/lib/hooks';
import { intelligenceToSignals } from '@/lib/intelligence/bridge';
import { useIntelConnected, useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
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
  const intelConnected = useIntelConnected();
  const whales = useWhales();
  const smart = useSmartMoney();

  const [typeFilter, setTypeFilter] = useState<SignalFilterType>('All');
  const [dirFilter, setDirFilter] = useState<SignalFilterDirection>('All');
  const [sortKey, setSortKey] = useState<SignalSortKey>('time');
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [showIntel, setShowIntel] = useState(false);

  const fetcher = useCallback(async () => {
    const live = await api.liveSignals({ limit: 200 });
    if (typeFilter === 'All') return live;
    return live.filter((s) => s.signal_type === typeFilter);
  }, [typeFilter]);
  const { data: historical, loading, error } = useFetch(fetcher, 15_000);
  const liveSignals = useStreamSignals<SignalEvent>(500);

  const intelSignals = useMemo(
    () => (showIntel ? intelligenceToSignals(whales, smart) : []),
    [whales, smart, showIntel],
  );

  const organized = useMemo(
    () => mergeSignals(liveSignals, historical, intelSignals),
    [liveSignals, historical, intelSignals],
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
    <div className="flex flex-col gap-5 max-w-[90rem] pb-8">
      <header className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100 tracking-tight">Signals</h1>
          <p className="text-platform-muted text-sm mt-0.5">
            Engine signals + live whale intelligence from chain
          </p>
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          <button
            type="button"
            onClick={() => setShowIntel((v) => !v)}
            className={`text-xs px-3 py-1.5 rounded-lg border transition-colors ${
              showIntel
                ? 'border-platform-accent/40 bg-platform-accent/10 text-platform-accent'
                : 'border-platform-border text-platform-muted hover:text-slate-200'
            }`}
          >
            Whale feed {showIntel ? 'on' : 'off'}
          </button>
          <span
            className={`text-[10px] px-2 py-1 rounded-md border ${
              intelConnected
                ? 'border-platform-accent/30 text-platform-accent'
                : 'border-red-500/30 text-red-400'
            }`}
          >
            Intel {intelConnected ? 'live' : 'offline'}
          </span>
        </div>
      </header>

      {error && (
        <p className="text-xs text-amber-400 bg-amber-500/10 border border-amber-500/20 rounded-lg px-3 py-2 max-w-xl">
          Historical API: {error}. Live feeds may still work.
        </p>
      )}

      <SignalStatsBar stats={organized.stats} connected={connected || intelConnected} />

      <div className="grid grid-cols-1 xl:grid-cols-[1fr_17rem] gap-4">
        <div className="flex flex-col gap-4 min-w-0">
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

        <aside className="flex flex-col gap-3 xl:sticky xl:top-0 xl:self-start">
          <WhaleFeed compact />
          <WalletRail />
        </aside>
      </div>

      <TxTape />
    </div>
  );
}
