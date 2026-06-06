'use client';

import { useCallback, useMemo, useState } from 'react';

import TxTape from '@/components/intelligence/TxTape';
import WalletRail from '@/components/intelligence/WalletRail';
import WhaleFeed from '@/components/intelligence/WhaleFeed';
import WhaleSourcesPanel from '@/components/intelligence/WhaleSourcesPanel';
import { PageHeader, PageShell } from '@/components/layout/PageShell';
import SignalDetailPanel from '@/components/signals/SignalDetailPanel';
import SignalFeedTable from '@/components/signals/SignalFeedTable';
import SignalFilters from '@/components/signals/SignalFilters';
import SignalStatsBar from '@/components/signals/SignalStatsBar';
import { useWsContext } from '@/components/WebSocketProvider';
import { api } from '@/lib/api';
import { useFetch, useStreamSignals } from '@/lib/hooks';
import { intelligenceToSignals } from '@/lib/intelligence/bridge';
import { useIntelConnected, useSmartMoney, useWhales } from '@/lib/intelligence/hooks';
import { streamSignalsToEvents } from '@/lib/intelligence/streamBridge';
import {
  filterSignals,
  mergeSignals,
  sortSignals,
  type SignalFilterDirection,
  type SignalFilterType,
  type SignalSortKey,
} from '@/lib/signals';
import type { SignalEvent } from '@/lib/types';
import { useMarketStore } from '@/stores/marketStore';
import { useStreamStore } from '@/stores/streamStore';

export default function SignalsPage() {
  const { connected } = useWsContext();
  const intelConnected = useIntelConnected();
  const streamConnected = useStreamStore((s) => s.connected);
  const marketSignals = useMarketStore((s) => s.signals);
  const whales = useWhales();
  const smart = useSmartMoney();

  const [typeFilter, setTypeFilter] = useState<SignalFilterType>('All');
  const [dirFilter, setDirFilter] = useState<SignalFilterDirection>('All');
  const [sortKey, setSortKey] = useState<SignalSortKey>('time');
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [showIntel, setShowIntel] = useState(true);

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

  const streamApiSignals = useMemo(
    () => (streamConnected ? streamSignalsToEvents(marketSignals) : []),
    [marketSignals, streamConnected],
  );

  const organized = useMemo(
    () => mergeSignals(liveSignals, historical, [...intelSignals, ...streamApiSignals]),
    [liveSignals, historical, intelSignals, streamApiSignals],
  );

  const filtered = useMemo(() => {
    const base = filterSignals(organized.signals, typeFilter, dirFilter);
    return sortSignals(base, sortKey);
  }, [organized.signals, typeFilter, dirFilter, sortKey]);

  const selected = useMemo(
    () => filtered.find((s) => s.signal_id === selectedId) ?? null,
    [filtered, selectedId],
  );

  const anyFeedLive = connected || intelConnected || streamConnected;

  return (
    <PageShell>
      <PageHeader
        title="Signals"
        description="Engine + stream-api + intelligence-api whale radar — unified feed"
        actions={
          <>
            <button
              type="button"
              onClick={() => setShowIntel((v) => !v)}
              className={`text-[11px] px-2.5 py-1 rounded-terminal border transition-colors ${
                showIntel
                  ? 'border-ds-blue/40 bg-ds-blue/10 text-ds-blue'
                  : 'border-ds-border text-ds-text-muted hover:text-ds-text-primary'
              }`}
            >
              Intel {showIntel ? 'on' : 'off'}
            </button>
            <span
              className={`text-[10px] px-2 py-1 rounded-terminal border font-mono ${
                intelConnected
                  ? 'border-ds-green/30 text-ds-green'
                  : 'border-ds-border text-ds-text-muted'
              }`}
            >
              :8090 {intelConnected ? 'live' : 'off'}
            </span>
            <span
              className={`text-[10px] px-2 py-1 rounded-terminal border font-mono ${
                streamConnected
                  ? 'border-ds-green/30 text-ds-green'
                  : 'border-ds-border text-ds-text-muted'
              }`}
            >
              :8080 {streamConnected ? 'live' : 'off'}
            </span>
          </>
        }
      />

      <WhaleSourcesPanel />

      {error && (
        <p className="text-xs text-ds-amber bg-ds-amber/10 border border-ds-amber/20 rounded-terminal px-3 py-2 max-w-xl">
          Historical API: {error}. Live feeds may still work.
        </p>
      )}

      {!intelConnected && (
        <p className="text-xs text-ds-text-secondary bg-ds-elevated/50 border border-ds-border rounded-terminal px-3 py-2 max-w-xl">
          Intelligence WebSocket offline — start{' '}
          <code className="text-ds-blue font-mono">cargo run -p intelligence-api</code> on :8090
          for whale / smart-money alerts.
        </p>
      )}

      <SignalStatsBar stats={organized.stats} connected={anyFeedLive} />

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
    </PageShell>
  );
}
