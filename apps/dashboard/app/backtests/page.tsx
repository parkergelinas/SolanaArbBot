'use client';

import { useEffect, useState } from 'react';

import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';

interface BacktestMetrics {
  strategy: string;
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  net_pnl_usd: number;
  win_rate: number;
  sharpe_approx?: number;
  max_drawdown_usd?: number;
}

interface PipelineReport {
  baseline?: { metrics?: { combined_net_pnl_usd?: number; scalp?: BacktestMetrics; arb?: BacktestMetrics } };
  retest?: { metrics?: { combined_net_pnl_usd?: number; scalp?: BacktestMetrics; arb?: BacktestMetrics } };
  simulation?: {
    net_pnl_usd?: number;
    trades_executed?: number;
    win_rate?: number;
    scalp_pnl_usd?: number;
    arb_pnl_usd?: number;
  };
}

export default function BacktestsPage() {
  const [report, setReport] = useState<PipelineReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch('/data/backtest_results.json')
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error('No results file'))))
      .then((data: PipelineReport) => setReport(data))
      .catch(() => setError(null));
  }, []);

  const sim = report?.simulation;
  const retest = report?.retest?.metrics;
  const hasData = !!(sim || retest);

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Backtests"
        subtitle="Dual-strategy pipeline — scalping + DEX arb on synthetic replay data"
        actions={
          <DsBadge tone={hasData ? 'green' : 'muted'}>{hasData ? 'Results loaded' : 'No data'}</DsBadge>
        }
      />

      <div className="flex items-center gap-2 px-3 py-2 bg-ds-elevated/40 border border-ds-border rounded-terminal text-[11px] text-ds-text-secondary shrink-0">
        <span className="text-[10px] uppercase tracking-wider text-ds-text-muted shrink-0">CLI</span>
        <code className="font-mono text-ds-green text-[10px] truncate">
          cargo run -p backtester-app -- --hours 6 --interval 10 -o data/backtest_results.json
        </code>
      </div>

      {hasData ? (
        <>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0">
            <DsStatPill
              label="Combined net"
              value={`$${(retest?.combined_net_pnl_usd ?? sim?.net_pnl_usd ?? 0).toFixed(2)}`}
              accent="var(--green)"
            />
            <DsStatPill label="Trades" value={sim?.trades_executed ?? '—'} />
            <DsStatPill
              label="Win rate"
              value={sim?.win_rate != null ? `${(sim.win_rate * 100).toFixed(1)}%` : '—'}
              accent="var(--blue)"
            />
            <DsStatPill
              label="Scalp PnL"
              value={`$${(sim?.scalp_pnl_usd ?? retest?.scalp?.net_pnl_usd ?? 0).toFixed(2)}`}
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-2 flex-1 min-h-0">
            {(['scalp', 'arb'] as const).map((key) => {
              const m = retest?.[key];
              if (!m) return null;
              return (
                <DsPanel key={key} title={key === 'scalp' ? 'Scalping (holdout)' : 'DEX Arb (holdout)'} compact>
                  <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-[11px]">
                    <div>
                      <dt className="text-ds-text-muted uppercase text-[9px]">Trades</dt>
                      <dd className="font-mono text-ds-text-primary">{m.total_trades}</dd>
                    </div>
                    <div>
                      <dt className="text-ds-text-muted uppercase text-[9px]">Net PnL</dt>
                      <dd className={`font-mono ${m.net_pnl_usd >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
                        ${m.net_pnl_usd.toFixed(2)}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-ds-text-muted uppercase text-[9px]">Win rate</dt>
                      <dd className="font-mono text-ds-text-primary">{(m.win_rate * 100).toFixed(1)}%</dd>
                    </div>
                    <div>
                      <dt className="text-ds-text-muted uppercase text-[9px]">W / L</dt>
                      <dd className="font-mono text-ds-text-secondary">
                        {m.winning_trades} / {m.losing_trades}
                      </dd>
                    </div>
                  </dl>
                </DsPanel>
              );
            })}
          </div>
        </>
      ) : (
        <DsPanel className="border-dashed flex-1 flex flex-col items-center justify-center py-12 space-y-3">
          <p className="text-[12px] text-ds-text-secondary">No backtest results loaded.</p>
          <p className="text-[10px] text-ds-text-muted max-w-md mx-auto leading-relaxed text-center">
            Run the CLI above from the workspace root. Output is written to{' '}
            <code className="text-ds-blue font-mono">data/backtest_results.json</code> — copy to{' '}
            <code className="text-ds-blue font-mono">apps/dashboard/public/data/</code> to view here.
          </p>
          {error && <p className="text-[10px] text-ds-amber">{error}</p>}
        </DsPanel>
      )}
    </PageShell>
  );
}
