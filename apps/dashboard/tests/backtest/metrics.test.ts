import { describe, expect, it } from 'vitest';

import {
  computeRoiMetrics,
  formatRoiPct,
  metricsFromRanking,
  rankingForConfig,
  simulateEnriched,
} from '@/lib/backtest/metrics';
import { DEFAULT_STRATEGY_CONFIG } from '@/lib/strategies/presets';

describe('backtest metrics bridge', () => {
  it('computes ROI from holdout window', () => {
    const roi = computeRoiMetrics(125, 6, 2500);
    expect(roi.windowRoiPct).toBeCloseTo(5, 1);
    expect(roi.dailyRoiPct).toBeCloseTo(20, 1);
    expect(roi.monthlyRoiPct).toBeCloseTo(600, 1);
    expect(formatRoiPct(roi.dailyRoiPct)).toMatch(/^\+/);
  });

  it('maps ranking rows to enriched metrics', () => {
    const row = {
      id: 'combined',
      name: 'Combined',
      description: 'test',
      win_probability: 0.9,
      pass_probability: 0.8,
      expected_usd_per_trade: 2,
      net_pnl_usd: 500,
      sharpe_approx: 2,
      trades_per_day: 100,
      max_drawdown_usd: 10,
      confidence_score: 88,
      verified: true,
      rank: 1,
      bot_store_keys: ['scalp', 'arb'],
    };
    const m = metricsFromRanking(row, 6);
    expect(m.confidenceScore).toBe(88);
    expect(m.matchingRankingId).toBe('combined');
    expect(m.dailyRoiPct).toBeGreaterThan(0);
  });

  it('finds closest ranking for preset config', () => {
    const report = {
      strategy_rankings: [
        {
          id: 'combined',
          bot_store_keys: ['scalp', 'arb'],
          confidence_score: 90,
          win_probability: 1,
          pass_probability: 0.9,
          expected_usd_per_trade: 1,
          net_pnl_usd: 100,
          sharpe_approx: 1,
          trades_per_day: 10,
          max_drawdown_usd: 0,
          name: 'Combined',
          description: '',
          verified: true,
          rank: 1,
        },
      ],
    };
    const match = rankingForConfig(
      { ...DEFAULT_STRATEGY_CONFIG, momentum: false },
      report,
    );
    expect(match?.id).toBe('combined');
  });

  it('enriches simulation with ROI fields', () => {
    const result = simulateEnriched(
      { ...DEFAULT_STRATEGY_CONFIG, whale_copy: false, momentum: false },
      null,
      6,
    );
    expect(result.dailyRoiPct).toBeDefined();
    expect(result.monthlyRoiPct).toBeDefined();
    expect(result.enabledEngines).toContain('scalp');
  });
});
