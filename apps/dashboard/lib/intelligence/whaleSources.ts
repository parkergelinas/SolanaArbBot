import type { WalletTier } from './types';

/** Default on-chain whale threshold (matches `WHALE_THRESHOLD_SOL` in data-layer). */
export const WHALE_THRESHOLD_SOL = 10;

/** Minimum swap size for smart-money alerts (intelligence-api detector). */
export const SMART_MONEY_MIN_SOL = 1;

/** Active trader tier floor (wallet tracker classification). */
export const ACTIVE_TIER_MIN_SOL = 2;

export type SignalFeedSource =
  | 'intelligence-api'
  | 'stream-api'
  | 'control-api'
  | 'dexscreener';

export interface WhaleSourceDescriptor {
  id: SignalFeedSource;
  label: string;
  endpoint: string;
  role: string;
  signals: string[];
}

/** Documented whale-tracking pipeline — mirrors `docs/data_layer.md` + intelligence-api. */
export const WHALE_DATA_SOURCES: WhaleSourceDescriptor[] = [
  {
    id: 'intelligence-api',
    label: 'Intelligence API',
    endpoint: 'ws://localhost:8090/intelligence',
    role: 'Primary whale radar — enriched swaps from Yellowstone/mock ingest',
    signals: ['whale_alert', 'smart_money_alert', 'wallet_snapshot', 'enriched_swap'],
  },
  {
    id: 'stream-api',
    label: 'Stream API',
    endpoint: 'ws://localhost:8080/stream',
    role: 'Market stream — fused signal kinds from swap tape',
    signals: ['whale_flow', 'smart_money', 'momentum', 'imbalance'],
  },
  {
    id: 'control-api',
    label: 'Control API',
    endpoint: 'ws://localhost:3001/ws',
    role: 'Engine signals — alpha fusion + historical REST',
    signals: ['WhaleFlow', 'SmartMoney', 'Momentum', 'Swap'],
  },
  {
    id: 'dexscreener',
    label: 'DexScreener',
    endpoint: 'https://api.dexscreener.com',
    role: 'Price anchors + token metadata for notional USD on swaps',
    signals: [],
  },
];

export interface WalletTierRule {
  tier: WalletTier;
  label: string;
  criteria: string;
}

export const WALLET_TIER_RULES: WalletTierRule[] = [
  {
    tier: 'whale',
    label: 'Whale',
    criteria: `Labeled whale OR swap ≥ ${WHALE_THRESHOLD_SOL} SOL`,
  },
  {
    tier: 'smart',
    label: 'Smart Money',
    criteria: `Labeled smart wallet + swap ≥ ${SMART_MONEY_MIN_SOL} SOL`,
  },
  {
    tier: 'active',
    label: 'Active',
    criteria: `Swap ≥ ${ACTIVE_TIER_MIN_SOL} SOL (rolling volume tracker)`,
  },
  {
    tier: 'retail',
    label: 'Retail',
    criteria: 'Below active thresholds — tracked but not alerted',
  },
];

export interface DetectionMethod {
  name: string;
  description: string;
}

export const WHALE_DETECTION_METHODS: DetectionMethod[] = [
  {
    name: 'Threshold detection',
    description: `Flag swaps where amount_sol ≥ WHALE_THRESHOLD_SOL (default ${WHALE_THRESHOLD_SOL}). Strength scales as tanh(amount / threshold).`,
  },
  {
    name: 'Label enrichment',
    description:
      'data-layer enrich stage attaches wallet_label from DashMap registry (whale_alpha, smart wallets) or heuristics on wallet prefix.',
  },
  {
    name: 'Smart-money gate',
    description: `wallet_label = smart AND amount_sol ≥ ${SMART_MONEY_MIN_SOL} SOL → SmartMoneyAlert with fixed strength 0.7.`,
  },
  {
    name: 'Wallet tracker',
    description:
      'Rolling per-wallet volume, net flow, and win_proxy score; emits WalletSnapshot on every enriched swap.',
  },
  {
    name: 'Alpha fusion (engine)',
    description:
      'control-api signal engine weights whale_activity 30%, momentum 20%, arb 25% — see docs/alpha_engine.md.',
  },
];

/** Classify wallet tier from swap size + optional label (mirrors wallet/tracker.rs). */
export function classifyWalletTier(
  amountSol: number,
  walletLabel?: string | null,
): WalletTier {
  if (walletLabel === 'whale') return 'whale';
  if (walletLabel === 'smart') return 'smart';
  if (amountSol >= WHALE_THRESHOLD_SOL) return 'whale';
  if (amountSol >= ACTIVE_TIER_MIN_SOL) return 'active';
  return 'retail';
}

/** Whether a swap would trigger a whale alert (mirrors whale/detector.rs). */
export function isWhaleSwap(amountSol: number, walletLabel?: string | null): boolean {
  return amountSol >= WHALE_THRESHOLD_SOL || walletLabel === 'whale';
}

/** Whether a swap would trigger smart-money alert. */
export function isSmartMoneySwap(amountSol: number, walletLabel?: string | null): boolean {
  return walletLabel === 'smart' && amountSol >= SMART_MONEY_MIN_SOL;
}

export function tierLabel(tier: WalletTier): string {
  return WALLET_TIER_RULES.find((r) => r.tier === tier)?.label ?? tier;
}

export function tierColor(tier: WalletTier): string {
  switch (tier) {
    case 'whale':
      return '#4da3ff';
    case 'smart':
      return '#a78bfa';
    case 'active':
      return '#00dfa8';
    default:
      return '#8b95a8';
  }
}
