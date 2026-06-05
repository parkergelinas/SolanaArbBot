'use client';

import { useCallback, useState } from 'react';
import MetricCard from '@/components/MetricCard';
import SystemStatusBadge from '@/components/SystemStatusBadge';
import SignalCard from '@/components/SignalCard';
import { useFetch, useWsEvents } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd, type SignalEvent } from '@/lib/types';

export default function OverviewPage() {
  const { data: health }    = useFetch(useCallback(() => api.health(),     []), 5_000);
  const { data: status }    = useFetch(useCallback(() => api.status(),     []), 5_000);
  const { data: portfolio } = useFetch(useCallback(() => api.portfolio(),  []), 10_000);
  const { data: risk }      = useFetch(useCallback(() => api.risk(),       []), 10_000);

  const liveSignals = useWsEvents<SignalEvent>('signal', 5);

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
    <div className="space-y-6 max-w-6xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">Overview</h1>
          <p className="text-slate-400 text-sm mt-0.5">System health &amp; portfolio at a glance</p>
        </div>
        <div className="flex items-center gap-4">
          <SystemStatusBadge />
          <div className="flex gap-2">
            <button
              onClick={handleStart}
              disabled={running || starting}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-green-500/10 text-green-400 border border-green-500/30 hover:bg-green-500/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ▶ Start
            </button>
            <button
              onClick={handleStop}
              disabled={!running || starting}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-red-500/10 text-red-400 border border-red-500/30 hover:bg-red-500/20 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              ■ Stop
            </button>
          </div>
        </div>
      </div>

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
          {liveSignals.length > 0 && (
            <span className="ml-2 text-xs text-slate-500">
              ({liveSignals.length} new via WebSocket)
            </span>
          )}
        </h2>
        {liveSignals.length === 0 ? (
          <div className="text-slate-500 text-sm bg-slate-800 border border-slate-700 rounded-xl p-6 text-center">
            No live signals yet — start the engine to begin.
          </div>
        ) : (
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-3">
            {[...liveSignals].reverse().map((s, i) => (
              <SignalCard key={`${s.signal_id}-${i}`} signal={s} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
