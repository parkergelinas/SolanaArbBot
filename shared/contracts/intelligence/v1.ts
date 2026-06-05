/**
 * Intelligence stream contract v1 — data layer → frontend boundary.
 */

export const INTELLIGENCE_SCHEMA_VERSION = 1;

export type Dex = 'raydium' | 'orca' | 'jupiter';
export type WalletTier = 'whale' | 'smart' | 'active' | 'retail';

/** Raw Yellowstone / Geyser update (ingestion output). */
export interface RawUpdate {
  slot: number;
  signature: string;
  logs: string[];
  accounts: string[];
  timestamp: number;
}

/** Canonical normalized swap (data-layer output). */
export interface SwapEvent {
  signature: string;
  wallet: string;
  token: string;
  amount_sol: number;
  dex: string;
  timestamp: number;
}

/** Enriched swap with wallet + token metadata. */
export interface EnrichedSwapEvent {
  v: typeof INTELLIGENCE_SCHEMA_VERSION;
  signature: string;
  wallet: string;
  token: string;
  amount_sol: number;
  dex: string;
  timestamp: number;
  token_symbol: string;
  wallet_label?: string | null;
  notional_usd: number;
  slot: number;
}

export interface WhaleAlert {
  v: typeof INTELLIGENCE_SCHEMA_VERSION;
  alert_id: string;
  signature: string;
  wallet: string;
  token: string;
  token_symbol: string;
  dex: Dex;
  amount_sol: number;
  notional_usd: number;
  strength: number;
  confidence: number;
  tier: WalletTier;
  timestamp: number;
  detail?: string;
}

export interface SmartMoneyAlert {
  v: typeof INTELLIGENCE_SCHEMA_VERSION;
  alert_id: string;
  signature: string;
  wallet: string;
  token: string;
  token_symbol: string;
  dex: Dex;
  amount_sol: number;
  notional_usd: number;
  strength: number;
  confidence: number;
  timestamp: number;
  detail?: string;
}

export interface WalletSnapshot {
  v: typeof INTELLIGENCE_SCHEMA_VERSION;
  wallet: string;
  tier: WalletTier;
  swap_count: number;
  volume_sol_24h: number;
  net_flow_sol: number;
  win_proxy: number;
  last_token: string;
  last_amount_sol: number;
  last_dex: string;
  timestamp: number;
}

export type IntelligenceMessage =
  | { type: 'swap'; payload: SwapEvent }
  | { type: 'enriched_swap'; payload: EnrichedSwapEvent }
  | { type: 'whale_alert'; payload: WhaleAlert }
  | { type: 'smart_money_alert'; payload: SmartMoneyAlert }
  | { type: 'wallet_snapshot'; payload: WalletSnapshot };

export interface IntelligenceBatch {
  v: typeof INTELLIGENCE_SCHEMA_VERSION;
  type: 'batch';
  seq: number;
  ts_ms: number;
  messages: IntelligenceMessage[];
}

export function isIntelligenceBatch(msg: unknown): msg is IntelligenceBatch {
  return (
    typeof msg === 'object' &&
    msg !== null &&
    (msg as IntelligenceBatch).type === 'batch' &&
    (msg as IntelligenceBatch).v === INTELLIGENCE_SCHEMA_VERSION &&
    Array.isArray((msg as IntelligenceBatch).messages)
  );
}
