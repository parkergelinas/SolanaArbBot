'use client';

import Link from 'next/link';

import DsBadge from '@/components/ui/DsBadge';
import { formatProbability } from '@/lib/backtest/applySelection';
import {
  confidenceTone,
  formatRoiPct,
  type EnrichedBacktestMetrics,
} from '@/lib/backtest/metrics';

interface BacktestOutlookBarProps {
  metrics: EnrichedBacktestMetrics | null;
  loaded: boolean;
  hours: number;
  compact?: boolean;
}

export default function BacktestOutlookBar({
  metrics,
  loaded,
  hours,
  compact,
}: BacktestOutlookBarProps) {
  if (!loaded) {
    return (
      <div className="rounded-terminal border border-ds-border bg-ds-elevated/20 px-3 py-2 animate-pulse h-14" />
    );
  }

  if (!metrics || metrics.enabledEngines.length === 0) {
    return (
      <p className="text-[10px] text-ds-amber px-1">
        Enable at least one engine to see backtest confidence and ROI estimates.
      </p>
    );
  }

  if (compact) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-[10px]">
        <DsBadge tone={confidenceTone(metrics.confidenceScore)}>
          {metrics.confidenceScore.toFixed(0)}% conf
        </DsBadge>
        <span className="font-mono text-ds-green">{formatRoiPct(metrics.dailyRoiPct)}/day</span>
        <span className="font-mono text-ds-text-muted">{formatRoiPct(metrics.monthlyRoiPct)}/mo</span>
      </div>
    );
  }

  return (
    <div className="rounded-terminal border border-ds-border bg-ds-elevated/30 p-3 space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <DsBadge tone={confidenceTone(metrics.confidenceScore)}>
          {metrics.confidenceScore.toFixed(0)}% confidence
        </DsBadge>
        {metrics.usedHoldoutData && (
          <span className="text-[9px] text-ds-green uppercase tracking-wider">Holdout data</span>
        )}
        {metrics.matchingRankingId && (
          <Link
            href="/backtests"
            className="text-[9px] text-ds-blue hover:underline font-mono"
          >
            ↔ {metrics.matchingRankingId} ranking
          </Link>
        )}
        <Link href="/backtests" className="text-[9px] text-ds-text-muted hover:text-ds-blue ml-auto">
          Full backtests →
        </Link>
      </div>
      <dl className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-6 gap-2 text-[10px]">
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">Win rate</dt>
          <dd className="font-mono text-ds-green">{formatProbability(metrics.winRate)}</dd>
        </div>
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">Pass prob</dt>
          <dd className="font-mono text-ds-text-primary">{formatProbability(metrics.passProbability)}</dd>
        </div>
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">{hours}h ROI</dt>
          <dd className={`font-mono ${metrics.windowRoiPct >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
            {formatRoiPct(metrics.windowRoiPct)}
          </dd>
        </div>
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">Est. daily</dt>
          <dd className={`font-mono ${metrics.dailyRoiPct >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
            {formatRoiPct(metrics.dailyRoiPct)}
          </dd>
        </div>
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">Est. monthly</dt>
          <dd className={`font-mono ${metrics.monthlyRoiPct >= 0 ? 'text-ds-green' : 'text-ds-red'}`}>
            {formatRoiPct(metrics.monthlyRoiPct)}
          </dd>
        </div>
        <div>
          <dt className="text-ds-text-muted uppercase text-[8px]">$/trade</dt>
          <dd className="font-mono text-ds-text-secondary">${metrics.expectedUsdPerTrade.toFixed(2)}</dd>
        </div>
      </dl>
    </div>
  );
}
