import type { BotStrategy } from '@/stores/botStore';

export interface StrategyRankingRow {
  id: string;
  name: string;
  description: string;
  win_probability: number;
  pass_probability: number;
  expected_usd_per_trade: number;
  net_pnl_usd: number;
  sharpe_approx: number;
  trades_per_day: number;
  max_drawdown_usd: number;
  confidence_score: number;
  verified: boolean;
  rank: number;
  bot_store_keys: string[];
}

const STORAGE_KEY = 'solarb_backtest_strategy_choice';

export function saveStrategyChoice(id: string): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    /* ignore */
  }
}

export function loadStrategyChoice(): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** Map a backtest ranking row to botStore strategy toggles. */
export function togglesForRanking(
  row: StrategyRankingRow,
): Partial<Record<BotStrategy, boolean>> {
  const next: Partial<Record<BotStrategy, boolean>> = {
    scalp: false,
    arb: false,
    whale_copy: false,
    momentum: false,
    sniper: false,
  };
  for (const key of row.bot_store_keys) {
    if (key in next) {
      next[key as BotStrategy] = true;
    }
  }
  return next;
}

export function formatProbability(p: number): string {
  return `${(p * 100).toFixed(1)}%`;
}
