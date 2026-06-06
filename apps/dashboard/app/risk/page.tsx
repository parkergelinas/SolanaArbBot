'use client';

import { useCallback } from 'react';

import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import { useFetch } from '@/lib/hooks';
import { api } from '@/lib/api';
import { formatUsd } from '@/lib/types';

function GaugeBar({ value, max, label }: { value: number; max: number; label: string }) {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const barColor = pct > 80 ? 'var(--red)' : pct > 60 ? 'var(--amber)' : 'var(--blue)';

  return (
    <div className="space-y-1.5">
      <div className="flex justify-between text-[10px] text-ds-text-muted uppercase tracking-wider">
        <span>{label}</span>
        <span className="font-mono tabular-nums">{pct.toFixed(1)}%</span>
      </div>
      <div className="h-1.5 bg-ds-elevated rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{ width: `${pct}%`, backgroundColor: barColor }}
        />
      </div>
    </div>
  );
}

export default function RiskPage() {
  const { data: risk } = useFetch(useCallback(() => api.risk(), []), 5_000);
  const { data: portfolio } = useFetch(useCallback(() => api.portfolio(), []), 5_000);

  const riskStatus = risk?.risk_status ?? 'unknown';
  const statusTone =
    riskStatus === 'healthy' ? 'green' : riskStatus === 'warning' ? 'amber' : 'red';

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Risk"
        subtitle="Current exposure vs configured limits"
        actions={<DsBadge tone={statusTone}>{riskStatus}</DsBadge>}
      />

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0">
        <DsStatPill label="Capital" value={risk ? formatUsd(risk.capital_usd) : '–'} />
        <DsStatPill
          label="Max drawdown"
          value={risk ? `${risk.max_drawdown_pct.toFixed(1)}%` : '–'}
          accent="var(--amber)"
        />
        <DsStatPill
          label="Max position"
          value={risk ? `${risk.max_position_pct.toFixed(1)}%` : '–'}
          accent="var(--blue)"
        />
        <DsStatPill
          label="Daily loss"
          value={risk ? formatUsd(risk.daily_loss_usd) : '–'}
          accent={risk && risk.daily_loss_usd > 0 ? 'var(--red)' : undefined}
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-2 flex-1 min-h-0">
        <DsPanel compact title="Exposure gauges">
          {risk ? (
            <div className="space-y-4">
              <GaugeBar label="Current exposure" value={risk.current_exposure_pct} max={100} />
              <GaugeBar
                label="Drawdown used"
                value={risk.daily_loss_usd}
                max={risk.capital_usd * (risk.max_drawdown_pct / 100)}
              />
            </div>
          ) : (
            <p className="text-ds-text-muted text-[11px] text-center py-6">
              No data — is the control API running?
            </p>
          )}
        </DsPanel>

        <DsPanel compact title="Portfolio summary">
          <div className="grid grid-cols-3 gap-2">
            {[
              { label: 'Open positions', value: String(portfolio?.open_positions ?? '–') },
              { label: 'Realised PnL', value: portfolio ? formatUsd(portfolio.realised_pnl) : '–' },
              { label: 'Win rate', value: portfolio ? `${(portfolio.win_rate * 100).toFixed(1)}%` : '–' },
            ].map(({ label, value }) => (
              <div
                key={label}
                className="bg-ds-elevated/40 border border-ds-border rounded-terminal px-2 py-2 text-center"
              >
                <p className="text-ds-text-muted text-[9px] uppercase tracking-wider">{label}</p>
                <p className="text-ds-text-primary font-medium font-mono text-[12px] mt-1 tabular-nums">
                  {value}
                </p>
              </div>
            ))}
          </div>
        </DsPanel>
      </div>

      <DsPanel compact title="Safety rules">
        <ul className="text-[11px] text-ds-text-muted space-y-1 font-mono">
          <li className="text-ds-text-secondary">Live trading disabled — paper mode only</li>
          <li>All config changes validated before application</li>
          <li>No direct execution access from the UI</li>
          <li>Portfolio reset restricted to paper mode</li>
        </ul>
      </DsPanel>
    </PageShell>
  );
}
