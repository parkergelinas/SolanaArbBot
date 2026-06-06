'use client';

import { useCallback, useMemo, useState } from 'react';

import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import LiveDataStatusCard from '@/components/LiveDataStatusCard';
import SignalCard from '@/components/SignalCard';
import SystemStatusBadge from '@/components/SystemStatusBadge';
import { useStreamLatest, useStreamSignals, useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd, type HealthStatus, type Portfolio, type Risk, type SignalEvent, type SystemStatus } from '@/lib/types';

export default function OverviewPage() {
  const health = useStreamLatest<HealthStatus>('health');
  const status = useStreamLatest<SystemStatus>('status');
  const portfolio = useStreamLatest<Portfolio>('portfolio');
  const risk = useStreamLatest<Risk>('risk');

  const liveSignals = useStreamSignals<SignalEvent>(5);
  const wsEmpty = liveSignals.length === 0;

  const fetchFallback = useCallback(() => api.liveSignals({ limit: 5 }), []);
  const { data: polledSignals } = useFetch(fetchFallback, wsEmpty ? 5_000 : null);

  const displaySignals = useMemo(() => {
    if (liveSignals.length > 0) return liveSignals;
    return polledSignals ?? [];
  }, [liveSignals, polledSignals]);

  const signalSource = liveSignals.length > 0 ? 'WebSocket' : polledSignals ? 'API poll' : null;

  const [starting, setStarting] = useState(false);

  const handleStart = async () => {
    setStarting(true);
    try {
      await api.startSystem();
    } finally {
      setStarting(false);
    }
  };
  const handleStop = async () => {
    setStarting(true);
    try {
      await api.stopSystem();
    } finally {
      setStarting(false);
    }
  };

  const running = status?.running ?? false;

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Overview"
        subtitle="System health & portfolio at a glance"
        actions={
          <>
            <SystemStatusBadge />
            <button
              onClick={handleStart}
              disabled={running || starting}
              className="px-2.5 py-1 rounded-terminal text-[11px] font-medium bg-ds-green/10 text-ds-green border border-ds-green/30 hover:bg-ds-green/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ▶ Start
            </button>
            <button
              onClick={handleStop}
              disabled={!running || starting}
              className="px-2.5 py-1 rounded-terminal text-[11px] font-medium bg-ds-red/10 text-ds-red border border-ds-red/30 hover:bg-ds-red/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ■ Stop
            </button>
          </>
        }
      />

      <div className="grid grid-cols-2 sm:grid-cols-4 xl:grid-cols-8 gap-2 shrink-0">
        <DsStatPill label="Capital" value={portfolio ? formatUsd(portfolio.capital_usd) : '–'} />
        <DsStatPill
          label="Unrealised"
          value={portfolio ? formatUsd(portfolio.unrealised_pnl) : '–'}
          accent={portfolio && portfolio.unrealised_pnl >= 0 ? 'var(--green)' : 'var(--red)'}
        />
        <DsStatPill
          label="Realised"
          value={portfolio ? formatUsd(portfolio.realised_pnl) : '–'}
          accent={portfolio && portfolio.realised_pnl >= 0 ? 'var(--green)' : 'var(--red)'}
        />
        <DsStatPill
          label="Win rate"
          value={portfolio ? `${(portfolio.win_rate * 100).toFixed(1)}%` : '–'}
          accent="var(--blue)"
        />
        <DsStatPill label="Signals" value={status?.signals_processed?.toLocaleString() ?? '–'} />
        <DsStatPill
          label="Uptime"
          value={health ? `${Math.floor(health.uptime_secs / 60)}m ${health.uptime_secs % 60}s` : '–'}
        />
        <DsStatPill
          label="Exposure"
          value={risk ? `${risk.current_exposure_pct.toFixed(1)}%` : '–'}
          accent={risk && risk.current_exposure_pct > 80 ? 'var(--red)' : undefined}
        />
        <DsStatPill
          label="Risk"
          value={risk?.risk_status ?? '–'}
          accent={risk?.risk_status === 'healthy' ? 'var(--green)' : 'var(--red)'}
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_18rem] gap-2 shrink-0">
        <LiveDataStatusCard compact />
        <DsPanel compact title="Portfolio">
          <dl className="grid grid-cols-2 gap-x-3 gap-y-2 text-[11px]">
            <div>
              <dt className="text-ds-text-muted uppercase text-[9px]">Trades</dt>
              <dd className="font-mono text-ds-text-primary">{portfolio?.total_trades ?? '–'}</dd>
            </div>
            <div>
              <dt className="text-ds-text-muted uppercase text-[9px]">Open</dt>
              <dd className="font-mono text-ds-text-primary">{portfolio?.open_positions ?? '–'}</dd>
            </div>
            <div>
              <dt className="text-ds-text-muted uppercase text-[9px]">Mode</dt>
              <dd className="font-mono text-ds-text-secondary capitalize">{status?.mode ?? '–'}</dd>
            </div>
            <div>
              <dt className="text-ds-text-muted uppercase text-[9px]">Engine</dt>
              <dd className={`font-mono capitalize ${running ? 'text-ds-green' : 'text-ds-text-muted'}`}>
                {running ? 'running' : 'stopped'}
              </dd>
            </div>
          </dl>
        </DsPanel>
      </div>

      <DsPanel
        className="flex-1 min-h-0 flex flex-col"
        flush
        title="Latest signals"
        action={
          displaySignals.length > 0 && signalSource ? (
            <span className="text-[10px] font-mono text-ds-text-muted">
              {displaySignals.length} via {signalSource}
            </span>
          ) : undefined
        }
      >
        {displaySignals.length === 0 ? (
          <div className="text-ds-text-muted text-[11px] p-8 text-center">
            No live signals yet — start the engine or open the terminal for market data.
          </div>
        ) : (
          <div className="p-2 grid md:grid-cols-2 xl:grid-cols-3 gap-2 overflow-y-auto terminal-scroll flex-1 min-h-0">
            {[...displaySignals].reverse().map((s, i) => (
              <SignalCard key={`${s.signal_id}-${i}`} signal={s} />
            ))}
          </div>
        )}
      </DsPanel>
    </PageShell>
  );
}
