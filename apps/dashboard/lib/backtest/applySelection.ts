import type { BotStrategy, BotStrategyConfig } from '@/stores/botStore';
import { cloneConfig } from '@/lib/strategies/presets';

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

/** Build a full strategy config from a backtest ranking row (toggles only). */
export function configFromRanking(
  row: StrategyRankingRow,
  base?: BotStrategyConfig,
): BotStrategyConfig {
  const toggles = togglesForRanking(row);
  const next = cloneConfig(base ?? {
    scalp: true,
    arb: true,
    whale_copy: false,
    momentum: true,
    sniper: false,
    min_confidence: 0.65,
    min_whale_sol: 50,
    auto_copy_whale: false,
    scalp_take_profit_pct: 0.012,
    scalp_stop_loss_pct: 0.006,
    arb_min_profit_usd: 0.25,
    arb_max_loss_usd: 2.0,
  });
  for (const key of Object.keys(toggles) as BotStrategy[]) {
    if (toggles[key] !== undefined) next[key] = toggles[key]!;
  }
  return next;
}
