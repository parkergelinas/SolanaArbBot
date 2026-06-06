import { create } from 'zustand';



import {

  cloneConfig,

  createCustomPresetId,

  DEFAULT_STRATEGY_CONFIG,

  loadActivePresetId,

  loadCustomPresets,

  saveActivePresetId,

  saveCustomPresets,

  type StrategyPreset,

} from '@/lib/strategies/presets';

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

  activePresetId: string;

  customPresets: StrategyPreset[];

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

  loadStrategies: (config: BotStrategyConfig) => void;

  applyPreset: (preset: StrategyPreset) => void;

  setActivePresetId: (id: string) => void;

  markCustomActive: () => void;

  saveCustomPreset: (name: string, description?: string) => StrategyPreset | null;

  deleteCustomPreset: (id: string) => void;

  refreshCustomPresets: () => void;

  hydrateFromApiConfig: (config: Record<string, unknown>) => void;

  recordWhaleCopy: () => void;

  toConfigPatch: () => Record<string, unknown>;

}



function asRecord(v: unknown): Record<string, unknown> | null {

  return v && typeof v === 'object' ? (v as Record<string, unknown>) : null;

}



function num(v: unknown, fallback: number): number {

  return typeof v === 'number' && Number.isFinite(v) ? v : fallback;

}



function bool(v: unknown, fallback: boolean): boolean {

  return typeof v === 'boolean' ? v : fallback;

}



export const useBotStore = create<BotStore>((set, get) => ({

  strategies: cloneConfig(DEFAULT_STRATEGY_CONFIG),

  activePresetId: typeof window !== 'undefined' ? loadActivePresetId() : 'balanced',

  customPresets: typeof window !== 'undefined' ? loadCustomPresets() : [],

  lastWhaleCopyAt: null,

  pendingCopies: 0,



  toggleStrategy: (key) =>

    set((s) => ({

      strategies: { ...s.strategies, [key]: !s.strategies[key] },

      activePresetId: 'custom-draft',

    })),



  setMinConfidence: (v) =>

    set((s) => ({ strategies: { ...s.strategies, min_confidence: v }, activePresetId: 'custom-draft' })),



  setMinWhaleSol: (v) =>

    set((s) => ({ strategies: { ...s.strategies, min_whale_sol: v }, activePresetId: 'custom-draft' })),



  setAutoCopyWhale: (v) =>

    set((s) => ({ strategies: { ...s.strategies, auto_copy_whale: v }, activePresetId: 'custom-draft' })),



  setScalpTakeProfitPct: (v) =>

    set((s) => ({ strategies: { ...s.strategies, scalp_take_profit_pct: v }, activePresetId: 'custom-draft' })),



  setScalpStopLossPct: (v) =>

    set((s) => ({ strategies: { ...s.strategies, scalp_stop_loss_pct: v }, activePresetId: 'custom-draft' })),



  setArbMinProfitUsd: (v) =>

    set((s) => ({ strategies: { ...s.strategies, arb_min_profit_usd: v }, activePresetId: 'custom-draft' })),



  setArbMaxLossUsd: (v) =>

    set((s) => ({ strategies: { ...s.strategies, arb_max_loss_usd: v }, activePresetId: 'custom-draft' })),



  loadStrategies: (config) =>

    set({ strategies: cloneConfig(config), activePresetId: 'custom-draft' }),



  applyPreset: (preset) => {

    saveActivePresetId(preset.id);

    set({

      strategies: cloneConfig(preset.config),

      activePresetId: preset.id,

    });

  },



  setActivePresetId: (id) => {

    saveActivePresetId(id);

    set({ activePresetId: id });

  },



  markCustomActive: () => set({ activePresetId: 'custom-draft' }),



  saveCustomPreset: (name, description) => {

    const trimmed = name.trim();

    if (!trimmed) return null;

    const preset: StrategyPreset = {

      id: createCustomPresetId(trimmed),

      kind: 'custom',

      name: trimmed,

      description: description?.trim() || 'Custom strategy preset',

      config: cloneConfig(get().strategies),

      createdAt: Date.now(),

    };

    const next = [...get().customPresets, preset];

    saveCustomPresets(next);

    saveActivePresetId(preset.id);

    set({ customPresets: next, activePresetId: preset.id });

    return preset;

  },



  deleteCustomPreset: (id) => {

    const next = get().customPresets.filter((p) => p.id !== id);

    saveCustomPresets(next);

    const active = get().activePresetId === id ? 'balanced' : get().activePresetId;

    if (active !== get().activePresetId) saveActivePresetId(active);

    set({ customPresets: next, activePresetId: active });

  },



  refreshCustomPresets: () => set({ customPresets: loadCustomPresets() }),



  hydrateFromApiConfig: (config) => {

    const strategy = asRecord(config.strategy);

    const signal = asRecord(config.signal_engine);

    const scalper = asRecord(config.scalper);

    const execution = asRecord(config.execution);

    if (!strategy && !signal && !scalper && !execution) return;



    const whaleUsd = num(signal?.whale_threshold_usd, 7500);

    const next: BotStrategyConfig = {

      scalp: bool(strategy?.scalp, DEFAULT_STRATEGY_CONFIG.scalp),

      arb: bool(strategy?.arb, DEFAULT_STRATEGY_CONFIG.arb),

      whale_copy: bool(strategy?.whale_copy, DEFAULT_STRATEGY_CONFIG.whale_copy),

      momentum: bool(strategy?.momentum, DEFAULT_STRATEGY_CONFIG.momentum),

      sniper: bool(strategy?.sniper, DEFAULT_STRATEGY_CONFIG.sniper),

      min_confidence: num(signal?.signal_min_confidence, DEFAULT_STRATEGY_CONFIG.min_confidence),

      min_whale_sol: Math.max(10, Math.round(whaleUsd / 150)),

      auto_copy_whale: get().strategies.auto_copy_whale,

      scalp_take_profit_pct: num(scalper?.take_profit_pct, DEFAULT_STRATEGY_CONFIG.scalp_take_profit_pct),

      scalp_stop_loss_pct: num(scalper?.stop_loss_pct, DEFAULT_STRATEGY_CONFIG.scalp_stop_loss_pct),

      arb_min_profit_usd: num(execution?.min_profit_threshold_usd, DEFAULT_STRATEGY_CONFIG.arb_min_profit_usd),

      arb_max_loss_usd: num(execution?.max_loss_per_trade_usd, DEFAULT_STRATEGY_CONFIG.arb_max_loss_usd),

    };

    set({ strategies: next });

  },



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


