import { describe, expect, it } from 'vitest';

import { simulateStrategyConfig } from '@/lib/backtest/simulateStrategy';
import { DEFAULT_STRATEGY_CONFIG } from '@/lib/strategies/presets';

describe('simulateStrategyConfig', () => {
  it('returns zero metrics when no engines enabled', () => {
    const result = simulateStrategyConfig(
      {
        ...DEFAULT_STRATEGY_CONFIG,
        scalp: false,
        arb: false,
        momentum: false,
      },
      null,
      6,
    );
    expect(result.netPnlUsd).toBe(0);
    expect(result.enabledEngines).toEqual([]);
  });

  it('estimates scalp+arb with holdout report', () => {
    const report = {
      retest: {
        metrics: {
          scalp: {
            strategy: 'scalp',
            total_trades: 100,
            winning_trades: 60,
            losing_trades: 40,
            net_pnl_usd: 50,
            win_rate: 0.6,
            avg_net_per_trade_usd: 0.5,
            trades_per_day: 400,
            sharpe_approx: 1.5,
          },
          arb: {
            strategy: 'arb',
            total_trades: 40,
            winning_trades: 30,
            losing_trades: 10,
            net_pnl_usd: 120,
            win_rate: 0.75,
            avg_net_per_trade_usd: 3,
            trades_per_day: 160,
            sharpe_approx: 2.5,
          },
        },
      },
    };
    const result = simulateStrategyConfig(DEFAULT_STRATEGY_CONFIG, report, 6);
    expect(result.enabledEngines).toContain('scalp');
    expect(result.enabledEngines).toContain('arb');
    expect(result.netPnlUsd).toBeGreaterThan(0);
    expect(result.usedHoldoutData).toBe(true);
  });
});
