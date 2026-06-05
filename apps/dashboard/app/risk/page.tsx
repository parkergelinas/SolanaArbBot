'use client';

import { useCallback } from 'react';
import MetricCard from '@/components/MetricCard';
import { useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd } from '@/lib/types';

function GaugeBar({ value, max, label, color }: { value: number; max: number; label: string; color: string }) {
  const pct = Math.min((value / max) * 100, 100);
  return (
    <div className="space-y-1.5">
      <div className="flex justify-between text-xs text-slate-400">
        <span>{label}</span>
        <span className="mono">{pct.toFixed(1)}%</span>
      </div>
      <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{ width: `${pct}%`, backgroundColor: pct > 80 ? '#ef4444' : pct > 60 ? '#eab308' : color }}
        />
      </div>
    </div>
  );
}

export default function RiskPage() {
  const { data: risk }      = useFetch(useCallback(() => api.risk(),      []), 5_000);
  const { data: portfolio } = useFetch(useCallback(() => api.portfolio(), []), 5_000);

  const riskStatus = risk?.risk_status ?? 'unknown';
  const statusColor = riskStatus === 'healthy' ? 'text-green-400' : riskStatus === 'warning' ? 'text-yellow-400' : 'text-red-400';

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">Risk Panel</h1>
          <p className="text-slate-400 text-sm mt-0.5">Current exposure vs configured limits</p>
        </div>
        <div className={`px-4 py-2 rounded-lg border text-sm font-medium capitalize ${
          riskStatus === 'healthy' ? 'bg-green-500/10 border-green-500/30 text-green-400' :
          riskStatus === 'warning' ? 'bg-yellow-500/10 border-yellow-500/30 text-yellow-400' :
          'bg-red-500/10 border-red-500/30 text-red-400'
        }`}>
          {riskStatus}
        </div>
      </div>

      {/* Limits grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <MetricCard
          label="Capital"
          value={risk ? formatUsd(risk.capital_usd) : '–'}
        />
        <MetricCard
          label="Max Drawdown"
          value={risk ? `${risk.max_drawdown_pct.toFixed(1)}%` : '–'}
          accent="yellow"
        />
        <MetricCard
          label="Max Position"
          value={risk ? `${risk.max_position_pct.toFixed(1)}%` : '–'}
          accent="blue"
        />
        <MetricCard
          label="Daily Loss"
          value={risk ? formatUsd(risk.daily_loss_usd) : '–'}
          accent={risk && risk.daily_loss_usd > 0 ? 'red' : 'default'}
        />
      </div>

      {/* Gauge bars */}
      <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-5">
        <h2 className="text-sm font-medium text-slate-300">Exposure Gauges</h2>
        {risk ? (
          <>
            <GaugeBar
              label="Current Exposure"
              value={risk.current_exposure_pct}
              max={100}
              color="#3b82f6"
            />
            <GaugeBar
              label="Drawdown Used"
              value={risk.daily_loss_usd}
              max={risk.capital_usd * (risk.max_drawdown_pct / 100)}
              color="#22c55e"
            />
          </>
        ) : (
          <p className="text-slate-500 text-sm text-center py-4">No data — is the control API running?</p>
        )}
      </div>

      {/* Portfolio summary */}
      <div className="bg-slate-800 border border-slate-700 rounded-xl p-5">
        <h2 className="text-sm font-medium text-slate-300 mb-4">Portfolio Summary</h2>
        <div className="grid grid-cols-3 gap-4 text-center">
          {[
            { label: 'Open Positions', value: String(portfolio?.open_positions ?? '–') },
            { label: 'Realised PnL',   value: portfolio ? formatUsd(portfolio.realised_pnl) : '–' },
            { label: 'Win Rate',        value: portfolio ? `${(portfolio.win_rate * 100).toFixed(1)}%` : '–' },
          ].map(({ label, value }) => (
            <div key={label} className="bg-slate-900/50 rounded-lg p-3">
              <p className="text-slate-500 text-xs">{label}</p>
              <p className="text-slate-200 font-medium mono text-sm mt-1">{value}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Safety rules notice */}
      <div className="bg-slate-800/50 border border-slate-700/50 rounded-xl p-4 text-xs text-slate-500 space-y-1">
        <p className="text-slate-400 font-medium mb-2">Safety Rules (enforced server-side)</p>
        <p>✓ Live trading disabled — paper mode only</p>
        <p>✓ All config changes validated before application</p>
        <p>✓ No direct execution access from the UI</p>
        <p>✓ Portfolio reset is restricted to paper mode</p>
      </div>
    </div>
  );
}
