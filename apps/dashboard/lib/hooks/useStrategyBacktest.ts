'use client';

import { useEffect, useMemo, useState } from 'react';

import {
  backtestHoursFromReport,
  enrichSimulation,
  fetchBacktestReport,
  metricsFromRanking,
  simulateEnriched,
  type EnrichedBacktestMetrics,
} from '@/lib/backtest/metrics';
import { loadStrategyChoice, type StrategyRankingRow } from '@/lib/backtest/applySelection';
import type { BacktestReportSlice } from '@/lib/backtest/simulateStrategy';
import type { BotStrategyConfig } from '@/stores/botStore';
import type { StrategyPreset } from '@/lib/strategies/presets';

export function useStrategyBacktest(config: BotStrategyConfig, activePresetId?: string) {
  const [report, setReport] = useState<BacktestReportSlice | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetchBacktestReport().then((data) => {
      if (!cancelled) {
        setReport(data);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const hours = useMemo(() => backtestHoursFromReport(report), [report]);
  const savedRankingId = useMemo(() => loadStrategyChoice(), []);

  const activeMetrics = useMemo((): EnrichedBacktestMetrics | null => {
    if (!loaded) return null;

    if (activePresetId?.startsWith('backtest-')) {
      const id = activePresetId.slice('backtest-'.length);
      const row = (report?.strategy_rankings as StrategyRankingRow[] | undefined)?.find(
        (r) => r.id === id,
      );
      if (row) return metricsFromRanking(row, hours);
    }

    if (savedRankingId && report?.strategy_rankings) {
      const row = (report.strategy_rankings as StrategyRankingRow[]).find(
        (r) => r.id === savedRankingId,
      );
      if (row) {
        const enabled = new Set<string>(
          ([
            config.scalp && 'scalp',
            config.arb && 'arb',
            config.whale_copy && 'whale_copy',
            config.momentum && 'momentum',
            config.sniper && 'sniper',
          ].filter(Boolean) as string[]),
        );
        const rowKeys = new Set(row.bot_store_keys);
        const sameKeys =
          enabled.size === rowKeys.size &&
          Array.from(enabled).every((k) => typeof k === 'string' && rowKeys.has(k));
        if (sameKeys) return metricsFromRanking(row, hours);
      }
    }

    return simulateEnriched(config, report, hours);
  }, [loaded, report, config, hours, activePresetId, savedRankingId]);

  function metricsForPreset(preset: StrategyPreset): EnrichedBacktestMetrics {
    return simulateEnriched(preset.config, report, hours);
  }

  return {
    report,
    loaded,
    hours,
    activeMetrics,
    metricsForPreset,
    savedRankingId,
  };
}
