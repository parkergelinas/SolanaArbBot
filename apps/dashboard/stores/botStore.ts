import { create } from 'zustand';

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
}

interface BotStore {
  strategies: BotStrategyConfig;
  lastWhaleCopyAt: number | null;
  pendingCopies: number;
  toggleStrategy: (key: keyof Omit<BotStrategyConfig, 'min_confidence' | 'min_whale_sol' | 'auto_copy_whale'>) => void;
  setMinConfidence: (v: number) => void;
  setMinWhaleSol: (v: number) => void;
  setAutoCopyWhale: (v: boolean) => void;
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

  recordWhaleCopy: () =>
    set((s) => ({
      lastWhaleCopyAt: Date.now(),
      pendingCopies: s.pendingCopies + 1,
    })),

  toConfigPatch: () => {
    const { strategies } = get();
    return {
      signal_engine: {
        whale_threshold_usd: Math.max(1000, strategies.min_whale_sol * 150),
        signal_min_confidence: strategies.min_confidence,
        signal_min_strength: strategies.momentum ? 0.25 : 0.4,
        smart_money_min_score: strategies.whale_copy ? 0.6 : 0.75,
      },
      features: {
        dry_run: true,
      },
      scalper: {
        signal_max_age_secs: strategies.momentum ? 120 : 45,
      },
    };
  },
}));
