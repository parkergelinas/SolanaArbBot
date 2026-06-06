'use client';

import { useCallback, useMemo, useState } from 'react';

import TxTape from '@/components/intelligence/TxTape';
import WalletRail from '@/components/intelligence/WalletRail';
import WhaleFeed from '@/components/intelligence/WhaleFeed';
import WhaleSourcesPanel from '@/components/intelligence/WhaleSourcesPanel';
import PumpFunScannerPanel from '@/components/signals/PumpFunScannerPanel';
import SignalDetailPanel from '@/components/signals/SignalDetailPanel';
import SignalFeedTable from '@/components/signals/SignalFeedTable';
import SignalFilters from '@/components/signals/SignalFilters';
import SignalStatsBar from '@/components/signals/SignalStatsBar';
import SolscanResearcherPanel from '@/components/signals/SolscanResearcherPanel';
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

function FeedPill({
  port,
  live,
  label,
}: {
  port: string;
  live: boolean;
  label?: string;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-terminal border text-[10px] font-mono uppercase ${
        live
          ? 'border-ds-green/30 text-ds-green bg-ds-green/5'
          : 'border-ds-border text-ds-text-muted bg-ds-elevated/30'
      }`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${live ? 'bg-ds-green live-pulse' : 'bg-ds-text-muted'}`} />
      {port} {label ?? (live ? 'live' : 'off')}
    </span>
  );
}

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
  const [mobileDeskTab, setMobileDeskTab] = useState<'feed' | 'inspect'>('feed');

  const handleSelectSignal = (id: number) => {
    setSelectedId(id);
    setMobileDeskTab('inspect');
  };

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
    <div className="flex flex-col gap-2 min-h-0 flex-1 max-w-[100rem] mx-auto w-full pb-4">
      {/* Header */}
      <header className="flex flex-wrap items-center gap-x-4 gap-y-2 shrink-0 pb-2 border-b border-ds-border">
        <div className="flex items-baseline gap-3 min-w-0">
          <h1 className="text-[15px] font-semibold tracking-[0.06em] text-ds-text-primary uppercase">
            Signals
          </h1>
          <p className="hidden sm:inline text-[11px] text-ds-text-muted truncate">
            Engine · stream-api · intelligence unified feed
          </p>
        </div>

        <div className="flex items-center gap-1.5 sm:gap-2 flex-wrap ml-auto w-full sm:w-auto">
          <button
            type="button"
            onClick={() => setShowIntel((v) => !v)}
            className={`text-[10px] px-2 py-0.5 rounded-terminal border font-medium uppercase tracking-wider transition-colors ${
              showIntel
                ? 'border-ds-blue/40 bg-ds-blue/10 text-ds-blue'
                : 'border-ds-border text-ds-text-muted hover:text-ds-text-primary hover:bg-ds-elevated/50'
            }`}
          >
            Intel {showIntel ? 'on' : 'off'}
          </button>
          <FeedPill port=":3001" live={connected} label={connected ? 'ctrl' : 'off'} />
          <FeedPill port=":8080" live={streamConnected} />
          <FeedPill port=":8090" live={intelConnected} />
        </div>
      </header>

      {/* Alerts */}
      {(error || !intelConnected) && (
        <div className="flex flex-col gap-1.5 shrink-0">
          {error && (
            <p className="text-[11px] text-ds-amber bg-ds-amber/8 border border-ds-amber/20 rounded-terminal px-3 py-1.5">
              Historical API: {error}. Live feeds may still work.
            </p>
          )}
          {!intelConnected && (
            <p className="text-[11px] text-ds-text-secondary bg-ds-elevated/40 border border-ds-border rounded-terminal px-3 py-1.5">
              Intelligence offline —{' '}
              <code className="text-ds-blue font-mono text-[10px]">cargo run -p intelligence-api</code>{' '}
              on :8090 for whale alerts.
            </p>
          )}
        </div>
      )}

      {/* Stats strip */}
      <SignalStatsBar stats={organized.stats} connected={anyFeedLive} />

      {/* Toolbar: filters + whale sources */}
      <div className="grid grid-cols-1 xl:grid-cols-[1fr_minmax(16rem,22rem)] gap-2 shrink-0 items-start">
        <SignalFilters
          type={typeFilter}
          direction={dirFilter}
          sort={sortKey}
          count={filtered.length}
          onType={setTypeFilter}
          onDirection={setDirFilter}
          onSort={setSortKey}
        />
        <WhaleSourcesPanel compact />
      </div>

      {/* Mobile desk tabs */}
      <div className="lg:hidden flex w-full p-0.5 gap-0.5 bg-ds-elevated border border-ds-border rounded-terminal shrink-0">
        <button
          type="button"
          onClick={() => setMobileDeskTab('feed')}
          className={`segment-btn flex-1 py-2 ${mobileDeskTab === 'feed' ? 'segment-btn-active' : ''}`}
        >
          Feed ({filtered.length})
        </button>
        <button
          type="button"
          onClick={() => setMobileDeskTab('inspect')}
          className={`segment-btn flex-1 py-2 ${mobileDeskTab === 'inspect' ? 'segment-btn-active' : ''}`}
        >
          Inspector
        </button>
      </div>

      {/* Main desk: table | detail | right rail */}
      <div className="shrink-0 min-h-[min(58dvh,520px)] lg:min-h-[min(62dvh,580px)] lg:flex-1 lg:min-h-0 grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_17rem] xl:grid-cols-[minmax(0,1fr)_17rem_14.5rem] grid-rows-1 gap-0 border border-ds-border rounded-terminal overflow-hidden bg-ds-base isolate">
        <div
          className={`flex flex-col min-h-0 min-w-0 h-full lg:col-span-1 xl:col-span-1 border-b lg:border-b-0 lg:border-r border-ds-border ${
            mobileDeskTab === 'feed' ? 'flex' : 'hidden lg:flex'
          }`}
        >
          <SignalFeedTable
            signals={filtered}
            liveIds={organized.liveIds}
            selectedId={selectedId}
            onSelect={handleSelectSignal}
            loading={loading}
          />
        </div>

        <div
          className={`flex flex-col min-h-0 min-w-0 h-full border-b xl:border-b-0 xl:border-r border-ds-border ${
            mobileDeskTab === 'inspect' ? 'flex' : 'hidden lg:flex'
          }`}
        >
          <SignalDetailPanel
            signal={selected}
            isLive={selected ? organized.liveIds.has(selected.signal_id) : false}
          />
        </div>

        <aside className="hidden xl:flex flex-col min-h-0 h-full divide-y divide-ds-border overflow-hidden">
          <div className="flex-1 min-h-0 overflow-hidden">
            <WhaleFeed compact fillHeight />
          </div>
          <div className="flex-1 min-h-0 overflow-hidden">
            <WalletRail fillHeight />
          </div>
        </aside>
      </div>

      {/* On-chain scanners: Solscan researcher + pump.fun edge */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-2 shrink-0 items-start">
        <SolscanResearcherPanel compact />
        <PumpFunScannerPanel compact />
      </div>

      {/* Mobile / tablet whale rail */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 xl:hidden shrink-0 items-start">
        <WhaleFeed compact />
        <WalletRail compact />
      </div>

      {/* Swap tape */}
      <TxTape />
    </div>
  );
}
