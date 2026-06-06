import { create } from 'zustand';

import { clusterToExpectedNetwork, rpcUrlForCluster } from '@/lib/solana/network';
import { useNetworkStore } from './networkStore';

export type BotStrategy =
  | 'scalp'
  | 'arb'
  | 'whale_copy'
  | 'momentum'
  | 'sniper';

export interface BotStrategyConfig {
  scalp: boolean;
  arb: boolean;
  whale_copy: boolean;
  momentum: boolean;
  sniper: boolean;
  min_confidence: number;
  min_whale_sol: number;
  auto_copy_whale: boolean;
  /** Scalper take-profit as fraction (0.012 = 1.2%). */
  scalp_take_profit_pct: number;
  /** Scalper stop-loss as fraction (0.006 = 0.6%). */
  scalp_stop_loss_pct: number;
  /** Min net profit per arb trade (USD). */
  arb_min_profit_usd: number;
  /** Max net loss per arb trade (USD). */
  arb_max_loss_usd: number;
}

interface BotStore {
  strategies: BotStrategyConfig;
  lastWhaleCopyAt: number | null;
  pendingCopies: number;
  toggleStrategy: (key: keyof Omit<BotStrategyConfig, 'min_confidence' | 'min_whale_sol' | 'auto_copy_whale' | 'scalp_take_profit_pct' | 'scalp_stop_loss_pct' | 'arb_min_profit_usd' | 'arb_max_loss_usd'>) => void;
  setMinConfidence: (v: number) => void;
  setMinWhaleSol: (v: number) => void;
  setAutoCopyWhale: (v: boolean) => void;
  setScalpTakeProfitPct: (v: number) => void;
  setScalpStopLossPct: (v: number) => void;
  setArbMinProfitUsd: (v: number) => void;
  setArbMaxLossUsd: (v: number) => void;
  recordWhaleCopy: () => void;
  toConfigPatch: () => Record<string, unknown>;
}

const DEFAULT: BotStrategyConfig = {
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
};

export const useBotStore = create<BotStore>((set, get) => ({
  strategies: { ...DEFAULT },
  lastWhaleCopyAt: null,
  pendingCopies: 0,

  toggleStrategy: (key) =>
    set((s) => ({
      strategies: { ...s.strategies, [key]: !s.strategies[key] },
    })),

  setMinConfidence: (v) =>
    set((s) => ({ strategies: { ...s.strategies, min_confidence: v } })),

  setMinWhaleSol: (v) =>
    set((s) => ({ strategies: { ...s.strategies, min_whale_sol: v } })),

  setAutoCopyWhale: (v) =>
    set((s) => ({ strategies: { ...s.strategies, auto_copy_whale: v } })),

  setScalpTakeProfitPct: (v) =>
    set((s) => ({ strategies: { ...s.strategies, scalp_take_profit_pct: v } })),

  setScalpStopLossPct: (v) =>
    set((s) => ({ strategies: { ...s.strategies, scalp_stop_loss_pct: v } })),

  setArbMinProfitUsd: (v) =>
    set((s) => ({ strategies: { ...s.strategies, arb_min_profit_usd: v } })),

  setArbMaxLossUsd: (v) =>
    set((s) => ({ strategies: { ...s.strategies, arb_max_loss_usd: v } })),

  recordWhaleCopy: () =>
    set((s) => ({
      lastWhaleCopyAt: Date.now(),
      pendingCopies: s.pendingCopies + 1,
    })),

  toConfigPatch: () => {
    const { strategies } = get();
    const cluster = useNetworkStore.getState().cluster;
    return {
      wallet: {
        rpc_endpoint: rpcUrlForCluster(cluster),
        expected_network: clusterToExpectedNetwork(cluster),
        validate_network_on_start: true,
      },
      strategy: {
        scalp: strategies.scalp,
        arb: strategies.arb,
        whale_copy: strategies.whale_copy,
        momentum: strategies.momentum,
        sniper: strategies.sniper,
      },
      signal_engine: {
        whale_threshold_usd: Math.max(1000, strategies.min_whale_sol * 150),
        signal_min_confidence: strategies.min_confidence,
        signal_min_strength: strategies.momentum ? 0.25 : 0.4,
        smart_money_min_score: strategies.whale_copy ? 0.6 : 0.75,
      },
      features: {
        dry_run: true,
        enable_live_trading: false,
      },
      scalper: {
        signal_max_age_secs: strategies.momentum ? 120 : 45,
        take_profit_pct: strategies.scalp_take_profit_pct,
        stop_loss_pct: strategies.scalp_stop_loss_pct,
      },
      execution: {
        min_profit_threshold_usd: strategies.arb_min_profit_usd,
        max_loss_per_trade_usd: strategies.arb_max_loss_usd,
      },
    };
  },
}));
