import type { BotStrategyConfig } from '@/stores/botStore';

export type StrategyPresetKind = 'builtin' | 'custom';

export interface StrategyPreset {
  id: string;
  kind: StrategyPresetKind;
  name: string;
  description: string;
  config: BotStrategyConfig;
  createdAt?: number;
}

export const DEFAULT_STRATEGY_CONFIG: BotStrategyConfig = {
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

export const BUILTIN_PRESETS: StrategyPreset[] = [
  {
    id: 'balanced',
    kind: 'builtin',
    name: 'Balanced',
    description: 'Scalp + arb with momentum signals — default paper desk.',
    config: { ...DEFAULT_STRATEGY_CONFIG },
  },
  {
    id: 'scalp_focus',
    kind: 'builtin',
    name: 'Scalp Focus',
    description: 'Short-horizon momentum scalps only; tight exits.',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: true,
      arb: false,
      whale_copy: false,
      momentum: true,
      sniper: false,
      scalp_take_profit_pct: 0.01,
      scalp_stop_loss_pct: 0.005,
      min_confidence: 0.6,
    },
  },
  {
    id: 'arb_focus',
    kind: 'builtin',
    name: 'DEX Arb',
    description: 'Cross-venue spread capture; higher min profit gate.',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: false,
      arb: true,
      whale_copy: false,
      momentum: false,
      sniper: false,
      arb_min_profit_usd: 0.35,
      arb_max_loss_usd: 1.5,
    },
  },
  {
    id: 'whale_hunter',
    kind: 'builtin',
    name: 'Whale Hunter',
    description: 'Whale copy + smart-money with auto paper copy.',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: false,
      arb: false,
      whale_copy: true,
      momentum: true,
      sniper: false,
      min_whale_sol: 30,
      min_confidence: 0.55,
      auto_copy_whale: true,
    },
  },
  {
    id: 'sniper_edge',
    kind: 'builtin',
    name: 'Sniper Edge',
    description: 'New-pool sniper with strict confidence filter.',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: false,
      arb: false,
      whale_copy: false,
      momentum: false,
      sniper: true,
      min_confidence: 0.75,
    },
  },
  {
    id: 'aggressive',
    kind: 'builtin',
    name: 'Aggressive Fusion',
    description: 'All engines on — max signal surface (paper only).',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: true,
      arb: true,
      whale_copy: true,
      momentum: true,
      sniper: true,
      min_confidence: 0.5,
      min_whale_sol: 20,
      scalp_take_profit_pct: 0.015,
      scalp_stop_loss_pct: 0.008,
      arb_min_profit_usd: 0.15,
    },
  },
  {
    id: 'conservative',
    kind: 'builtin',
    name: 'Conservative',
    description: 'High confidence, smaller arb edge, tight scalp stops.',
    config: {
      ...DEFAULT_STRATEGY_CONFIG,
      scalp: true,
      arb: true,
      whale_copy: false,
      momentum: false,
      sniper: false,
      min_confidence: 0.8,
      scalp_take_profit_pct: 0.008,
      scalp_stop_loss_pct: 0.004,
      arb_min_profit_usd: 0.5,
      arb_max_loss_usd: 1.0,
    },
  },
];

const CUSTOM_STORAGE_KEY = 'solarb_custom_strategy_presets';
const ACTIVE_PRESET_KEY = 'solarb_active_strategy_preset';

export function cloneConfig(config: BotStrategyConfig): BotStrategyConfig {
  return { ...config };
}

export function loadCustomPresets(): StrategyPreset[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(CUSTOM_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as StrategyPreset[];
    return Array.isArray(parsed) ? parsed.filter((p) => p.kind === 'custom' && p.config) : [];
  } catch {
    return [];
  }
}

export function saveCustomPresets(presets: StrategyPreset[]): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(CUSTOM_STORAGE_KEY, JSON.stringify(presets));
  } catch {
    /* ignore */
  }
}

export function loadActivePresetId(): string {
  if (typeof window === 'undefined') return 'balanced';
  try {
    return localStorage.getItem(ACTIVE_PRESET_KEY) ?? 'balanced';
  } catch {
    return 'balanced';
  }
}

export function saveActivePresetId(id: string): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(ACTIVE_PRESET_KEY, id);
  } catch {
    /* ignore */
  }
}

export function allPresets(custom: StrategyPreset[] = loadCustomPresets()): StrategyPreset[] {
  return [...BUILTIN_PRESETS, ...custom];
}

export function findPreset(id: string, custom?: StrategyPreset[]): StrategyPreset | undefined {
  return allPresets(custom).find((p) => p.id === id);
}

export function presetDisplayName(id: string, custom?: StrategyPreset[]): string {
  if (id === 'custom-draft') return 'Custom draft';
  if (id.startsWith('backtest-')) {
    const slug = id.slice('backtest-'.length).replace(/-/g, ' ');
    return slug ? `Backtest · ${slug}` : 'Backtest config';
  }
  return findPreset(id, custom)?.name ?? 'Unknown preset';
}

export function createCustomPresetId(name: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 32);
  const nonce = Math.random().toString(36).slice(2, 8);
  return `custom-${slug || 'strat'}-${Date.now().toString(36)}-${nonce}`;
}

export function enabledStrategyLabels(config: BotStrategyConfig): string[] {
  const labels: Record<string, string> = {
    scalp: 'Scalp',
    arb: 'Arb',
    whale_copy: 'Whale',
    momentum: 'Momentum',
    sniper: 'Sniper',
  };
  return (Object.keys(labels) as (keyof typeof labels)[]).filter(
    (k) => config[k as keyof BotStrategyConfig] === true,
  ).map((k) => labels[k]);
}
