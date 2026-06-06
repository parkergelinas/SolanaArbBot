import { describe, expect, it } from 'vitest';

import {
  configFromRanking,
  formatProbability,
  togglesForRanking,
  type StrategyRankingRow,
} from '@/lib/backtest/applySelection';

const sampleRow: StrategyRankingRow = {
  id: 'combined',
  name: 'Combined',
  description: 'test',
  win_probability: 0.82,
  pass_probability: 0.91,
  expected_usd_per_trade: 1.2,
  net_pnl_usd: 500,
  sharpe_approx: 2.1,
  trades_per_day: 400,
  max_drawdown_usd: 10,
  confidence_score: 88,
  verified: true,
  rank: 1,
  bot_store_keys: ['scalp', 'arb'],
};

describe('backtest strategy selection', () => {
  it('maps ranking to bot store toggles', () => {
    expect(togglesForRanking(sampleRow)).toEqual({
      scalp: true,
      arb: true,
      whale_copy: false,
      momentum: false,
      sniper: false,
    });
  });

  it('formats probability as percent', () => {
    expect(formatProbability(0.825)).toBe('82.5%');
  });

  it('builds full config from ranking', () => {
    const cfg = configFromRanking(sampleRow);
    expect(cfg.scalp).toBe(true);
    expect(cfg.arb).toBe(true);
    expect(cfg.whale_copy).toBe(false);
  });
});
