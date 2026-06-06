'use client';

import { useCallback, useMemo, useState } from 'react';
import { PageHeader, PageShell } from '@/components/layout/PageShell';
import LiveDataStatusCard from '@/components/LiveDataStatusCard';
import MetricCard from '@/components/MetricCard';
import SystemStatusBadge from '@/components/SystemStatusBadge';
import SignalCard from '@/components/SignalCard';
import { useStreamLatest, useStreamSignals, useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd, type HealthStatus, type Portfolio, type Risk, type SignalEvent, type SystemStatus } from '@/lib/types';

export default function OverviewPage() {
  const health    = useStreamLatest<HealthStatus>('health');
  const status    = useStreamLatest<SystemStatus>('status');
  const portfolio = useStreamLatest<Portfolio>('portfolio');
  const risk      = useStreamLatest<Risk>('risk');

  const liveSignals = useStreamSignals<SignalEvent>(5);
  const wsEmpty = liveSignals.length === 0;

  const fetchFallback = useCallback(
    () => api.liveSignals({ limit: 5 }),
    [],
  );
  const { data: polledSignals } = useFetch(fetchFallback, wsEmpty ? 5_000 : null);

  const displaySignals = useMemo(() => {
    if (liveSignals.length > 0) return liveSignals;
    return polledSignals ?? [];
  }, [liveSignals, polledSignals]);

  const signalSource = liveSignals.length > 0 ? 'WebSocket' : polledSignals ? 'API poll' : null;

  const [starting, setStarting] = useState(false);

  const handleStart = async () => {
    setStarting(true);
    try { await api.startSystem(); } finally { setStarting(false); }
  };
  const handleStop = async () => {
    setStarting(true);
    try { await api.stopSystem(); } finally { setStarting(false); }
  };

  const running = status?.running ?? false;

  return (
    <PageShell className="max-w-6xl">
      <PageHeader
        title="Overview"
        description="System health & portfolio at a glance"
        actions={
          <>
            <SystemStatusBadge />
            <button
              onClick={handleStart}
              disabled={running || starting}
              className="px-3 py-1.5 rounded-terminal text-sm font-medium bg-ds-green/10 text-ds-green border border-ds-green/30 hover:bg-ds-green/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ▶ Start
            </button>
            <button
              onClick={handleStop}
              disabled={!running || starting}
              className="px-3 py-1.5 rounded-terminal text-sm font-medium bg-ds-red/10 text-ds-red border border-ds-red/30 hover:bg-ds-red/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ■ Stop
            </button>
          </>
        }
      />

      <LiveDataStatusCard />

      {/* KPI grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <MetricCard
          label="Capital"
          value={portfolio ? formatUsd(portfolio.capital_usd) : '–'}
          accent="default"
        />
        <MetricCard
          label="Unrealised PnL"
          value={portfolio ? formatUsd(portfolio.unrealised_pnl) : '–'}
          accent={portfolio && portfolio.unrealised_pnl >= 0 ? 'green' : 'red'}
        />
        <MetricCard
          label="Realised PnL"
          value={portfolio ? formatUsd(portfolio.realised_pnl) : '–'}
          accent={portfolio && portfolio.realised_pnl >= 0 ? 'green' : 'red'}
        />
        <MetricCard
          label="Win Rate"
          value={portfolio ? `${(portfolio.win_rate * 100).toFixed(1)}%` : '–'}
          sub={`${portfolio?.total_trades ?? 0} trades`}
          accent="blue"
        />
        <MetricCard
          label="Signals Processed"
          value={status?.signals_processed?.toLocaleString() ?? '–'}
          accent="purple"
        />
        <MetricCard
          label="Uptime"
          value={health ? `${Math.floor(health.uptime_secs / 60)}m ${health.uptime_secs % 60}s` : '–'}
          accent="default"
        />
        <MetricCard
          label="Exposure"
          value={risk ? `${risk.current_exposure_pct.toFixed(1)}%` : '–'}
          sub={`Max ${risk?.max_drawdown_pct.toFixed(0)}% drawdown allowed`}
          accent={risk && risk.current_exposure_pct > 80 ? 'red' : 'default'}
        />
        <MetricCard
          label="Risk Status"
          value={risk?.risk_status ?? '–'}
          accent={risk?.risk_status === 'healthy' ? 'green' : 'red'}
        />
      </div>

      {/* Live signal feed */}
      <div>
        <h2 className="text-sm font-medium text-slate-300 mb-3">
          Latest Signals
          {displaySignals.length > 0 && signalSource && (
            <span className="ml-2 text-xs text-slate-500">
              ({displaySignals.length} via {signalSource})
            </span>
          )}
        </h2>
        {displaySignals.length === 0 ? (
          <div className="text-slate-500 text-sm bg-slate-800 border border-slate-700 rounded-xl p-6 text-center">
            No live signals yet — start the engine or open the terminal for market data.
          </div>
        ) : (
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-3">
            {[...displaySignals].reverse().map((s, i) => (
              <SignalCard key={`${s.signal_id}-${i}`} signal={s} />
            ))}
          </div>
        )}
      </div>
    </PageShell>
  );
}
