'use client';

import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';

import { CompactPageHeader, DsPanel, DsStatPill, PageShell } from '@/components/layout/PageShell';
import DsBadge from '@/components/ui/DsBadge';
import {
  configFromRanking,
  formatProbability,
  loadStrategyChoice,
  saveStrategyChoice,
  type StrategyRankingRow,
} from '@/lib/backtest/applySelection';
import {
  backtestHoursFromReport,
  computeRoiMetrics,
  confidenceTone,
  formatRoiPct,
} from '@/lib/backtest/metrics';
import { api } from '@/lib/api';
import { useBotStore } from '@/stores/botStore';

interface BacktestMetrics {
  strategy: string;
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  net_pnl_usd: number;
  win_rate: number;
  sharpe_approx?: number;
  max_drawdown_usd?: number;
  avg_net_per_trade_usd?: number;
  trades_per_day?: number;
}

interface PipelineReport {
  generated_at?: string;
  baseline?: { metrics?: { combined_net_pnl_usd?: number; scalp?: BacktestMetrics; arb?: BacktestMetrics } };
  retest?: { metrics?: { combined_net_pnl_usd?: number; scalp?: BacktestMetrics; arb?: BacktestMetrics } };
  simulation?: {
    net_pnl_usd?: number;
    trades_executed?: number;
    win_rate?: number;
    scalp_pnl_usd?: number;
    arb_pnl_usd?: number;
    duration_hours?: number;
  };
  quote_arb_holdout?: BacktestMetrics;
  strategy_rankings?: StrategyRankingRow[];
  retest_verification?: { passed: boolean };
}

export default function BacktestsPage() {
  const [report, setReport] = useState<PipelineReport | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [appliedId, setAppliedId] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const loadStrategies = useBotStore((s) => s.loadStrategies);
  const setActivePresetId = useBotStore((s) => s.setActivePresetId);
  const strategies = useBotStore((s) => s.strategies);

  useEffect(() => {
    fetch('/data/backtest_results.json')
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error('No results file'))))
      .then((data: PipelineReport) => {
        setReport(data);
        const saved = loadStrategyChoice();
        if (saved && data.strategy_rankings?.some((r) => r.id === saved)) {
          setSelectedId(saved);
        } else if (data.strategy_rankings?.length) {
          setSelectedId(data.strategy_rankings[0].id);
        }
      })
      .catch(() => setReport(null));
  }, []);

  const applySelection = useCallback(
    async (row: StrategyRankingRow) => {
      setApplying(true);
      setApplyError(null);
      try {
        const config = configFromRanking(row, strategies);
        loadStrategies(config);
        setActivePresetId(`backtest-${row.id}`);
        saveStrategyChoice(row.id);
        setSelectedId(row.id);
        setAppliedId(row.id);
        await api.patchConfig(useBotStore.getState().toConfigPatch());
      } catch (e) {
        setApplyError(String(e));
      } finally {
        setApplying(false);
      }
    },
    [loadStrategies, setActivePresetId, strategies],
  );

  const rankings = report?.strategy_rankings ?? [];
  const sim = report?.simulation;
  const retest = report?.retest?.metrics;
  const hasData = rankings.length > 0 || !!(sim || retest);
  const holdoutHours = backtestHoursFromReport(report);

  const generatedLabel = report?.generated_at
    ? new Date(Number(report.generated_at) * 1000).toLocaleString()
    : null;

  return (
    <PageShell desk>
      <CompactPageHeader
        title="Backtests"
        subtitle="Compare strategy win/pass probabilities on holdout replay — pick what to run live"
        actions={
          <div className="flex items-center gap-2">
            {report?.retest_verification && (
              <DsBadge tone={report.retest_verification.passed ? 'green' : 'amber'}>
                Holdout {report.retest_verification.passed ? 'verified' : 'unverified'}
              </DsBadge>
            )}
            <DsBadge tone={hasData ? 'green' : 'muted'}>{hasData ? 'Results loaded' : 'No data'}</DsBadge>
          </div>
        }
      />

      <div className="flex flex-wrap items-center gap-2 px-3 py-2 bg-ds-elevated/40 border border-ds-border rounded-terminal text-[11px] text-ds-text-secondary shrink-0">
        <span className="text-[10px] uppercase tracking-wider text-ds-text-muted shrink-0">CLI</span>
        <code className="font-mono text-ds-green text-[10px] truncate">
          cargo run -p backtester-app -- --hours 6 --interval 10 -o data/backtest_results.json
        </code>
        {generatedLabel && (
          <span className="text-ds-text-muted text-[10px] ml-auto">Run {generatedLabel}</span>
        )}
      </div>

      {hasData ? (
        <>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 shrink-0">
            <DsStatPill
              label="Best confidence"
              value={rankings[0] ? `${rankings[0].confidence_score.toFixed(0)}` : '—'}
              accent="var(--green)"
            />
            <DsStatPill
              label="Holdout net"
              value={`$${(retest?.combined_net_pnl_usd ?? sim?.net_pnl_usd ?? 0).toFixed(0)}`}
            />
            <DsStatPill
              label="Forward sim"
              value={`$${(sim?.net_pnl_usd ?? 0).toFixed(0)}`}
              accent="var(--blue)"
            />
            <DsStatPill
              label="Strategies ranked"
              value={rankings.length || '—'}
            />
          </div>

          <DsPanel title="Strategy comparison — choose by probability" compact className="shrink-0">
            <p className="text-[10px] text-ds-text-muted mb-3 leading-relaxed">
              Rankings use holdout win rate, verification pass probability, Sharpe, and per-trade edge.
              ROI % is scaled from holdout PnL vs ${2500} paper notional. Apply a row to sync confidence
              and ROI on the{' '}
              <Link href="/bot" className="text-ds-blue hover:underline">
                Bot
              </Link>{' '}
              page (paper mode).
            </p>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
              {rankings.map((row) => {
                const selected = selectedId === row.id;
                const applied = appliedId === row.id;
                const roi = computeRoiMetrics(row.net_pnl_usd, holdoutHours);
                return (
                  <button
                    key={row.id}
                    type="button"
                    onClick={() => setSelectedId(row.id)}
                    className={`text-left rounded-terminal border p-3 transition-colors ${
                      selected
                        ? 'border-ds-blue bg-ds-blue/10'
                        : 'border-ds-border bg-ds-elevated/30 hover:border-ds-border-strong'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <div>
                        <span className="text-[11px] font-medium text-ds-text-primary">{row.name}</span>
                        <span className="ml-2 text-[9px] text-ds-text-muted font-mono">#{row.rank}</span>
                      </div>
                      <DsBadge tone={confidenceTone(row.confidence_score)}>
                        {row.confidence_score.toFixed(0)} conf
                      </DsBadge>
                    </div>
                    <p className="text-[10px] text-ds-text-secondary mb-2 line-clamp-2">{row.description}</p>
                    <dl className="grid grid-cols-3 gap-2 text-[10px]">
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Win prob</dt>
                        <dd className="font-mono text-ds-green">{formatProbability(row.win_probability)}</dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Pass prob</dt>
                        <dd className="font-mono text-ds-text-primary">
                          {formatProbability(row.pass_probability)}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Daily ROI</dt>
                        <dd className="font-mono text-ds-green">{formatRoiPct(roi.dailyRoiPct)}</dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Monthly ROI</dt>
                        <dd className="font-mono text-ds-text-primary">{formatRoiPct(roi.monthlyRoiPct)}</dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">$/trade</dt>
                        <dd className="font-mono text-ds-text-primary">
                          ${row.expected_usd_per_trade.toFixed(2)}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Holdout PnL</dt>
                        <dd
                          className={`font-mono ${row.net_pnl_usd >= 0 ? 'text-ds-green' : 'text-ds-red'}`}
                        >
                          ${row.net_pnl_usd.toFixed(0)}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Trades/day</dt>
                        <dd className="font-mono text-ds-text-secondary">{row.trades_per_day.toFixed(0)}</dd>
                      </div>
                      <div>
                        <dt className="text-ds-text-muted uppercase text-[8px]">Sharpe</dt>
                        <dd className="font-mono text-ds-text-secondary">{row.sharpe_approx.toFixed(1)}</dd>
                      </div>
                    </dl>
                    {applied && (
                      <p className="mt-2 text-[9px] text-ds-green uppercase tracking-wider">Applied to bot</p>
                    )}
                  </button>
                );
              })}
            </div>
            {selectedId && rankings.find((r) => r.id === selectedId) && (
              <div className="mt-3 flex items-center gap-2 pt-3 border-t border-ds-border">
                <button
                  type="button"
                  onClick={() => {
                    const row = rankings.find((r) => r.id === selectedId);
                    if (row) void applySelection(row);
                  }}
                  disabled={applying}
                  className="px-3 py-1.5 rounded-terminal bg-ds-blue text-white text-[11px] font-medium hover:opacity-90 disabled:opacity-50"
                >
                  {applying ? 'Applying…' : `Apply ${rankings.find((r) => r.id === selectedId)?.name} to bot`}
                </button>
                <span className="text-[10px] text-ds-text-muted">
                  Enables:{' '}
                  {rankings
                    .find((r) => r.id === selectedId)
                    ?.bot_store_keys.join(', ') ?? '—'}
                </span>
                {applyError && (
                  <p className="text-[10px] text-ds-red w-full">{applyError}</p>
                )}
              </div>
            )}
          </DsPanel>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2 flex-1 min-h-0">
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
            {report?.quote_arb_holdout && (
              <DsPanel title="Route divergence (holdout)" compact>
                <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-[11px]">
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Opportunities</dt>
                    <dd className="font-mono text-ds-text-primary">{report.quote_arb_holdout.total_trades}</dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Net PnL</dt>
                    <dd className="font-mono text-ds-green">
                      ${report.quote_arb_holdout.net_pnl_usd.toFixed(2)}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Win rate</dt>
                    <dd className="font-mono text-ds-text-primary">
                      {(report.quote_arb_holdout.win_rate * 100).toFixed(1)}%
                    </dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Avg edge</dt>
                    <dd className="font-mono text-ds-text-secondary">
                      ${(report.quote_arb_holdout.avg_net_per_trade_usd ?? 0).toFixed(2)}
                    </dd>
                  </div>
                </dl>
              </DsPanel>
            )}
            {sim && (
              <DsPanel title="Forward simulation" compact>
                <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-[11px]">
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Net PnL</dt>
                    <dd className="font-mono text-ds-green">${sim.net_pnl_usd?.toFixed(2) ?? '—'}</dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Trades</dt>
                    <dd className="font-mono text-ds-text-primary">{sim.trades_executed ?? '—'}</dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Scalp slice</dt>
                    <dd className="font-mono text-ds-text-secondary">${sim.scalp_pnl_usd?.toFixed(0) ?? '—'}</dd>
                  </div>
                  <div>
                    <dt className="text-ds-text-muted uppercase text-[9px]">Arb slice</dt>
                    <dd className="font-mono text-ds-text-secondary">${sim.arb_pnl_usd?.toFixed(0) ?? '—'}</dd>
                  </div>
                </dl>
              </DsPanel>
            )}
          </div>
        </>
      ) : (
        <DsPanel className="border-dashed flex-1 flex flex-col items-center justify-center py-12 space-y-3">
          <p className="text-[12px] text-ds-text-secondary">No backtest results loaded.</p>
          <p className="text-[10px] text-ds-text-muted max-w-md mx-auto leading-relaxed text-center">
            Run the CLI above from the workspace root. Results are copied to{' '}
            <code className="text-ds-blue font-mono">apps/dashboard/public/data/backtest_results.json</code>.
          </p>
        </DsPanel>
      )}
    </PageShell>
  );
}
