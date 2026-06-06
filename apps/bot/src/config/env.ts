/** Runtime configuration — defaults use current Jupiter `api.jup.ag` endpoints. */

export interface BotEnv {
  jupiterSwapBase: string;
  jupiterPriceUrl: string;
  jupiterTokensBase: string;
  jupiterApiKey?: string;
  rpcUrl: string;
  slippageBps: number;
  minProfitUsd: number;
  tradeAmountUi: number;
  scanIntervalMs: number;
  paperMode: boolean;
  walletPublicKey?: string;
  /** Primary strategy id — `route_divergence_arb` | `round_trip_quote_arb` */
  primaryStrategy: string;
  enableMeanReversion: boolean;
  /** Stub only — Jupiter Trigger API (TP/SL/breakout). */
  enableTriggerApi: boolean;
  /** Stub only — Jupiter Recurring API (DCA/treasury). */
  enableRecurringApi: boolean;
  /** CoinMarketCap Pro API key for top-100 pair loading. */
  cmcApiKey?: string;
  /** Pair source: `cmc` (live API) or `static` (built-in map). */
  pairSource: 'cmc' | 'static';
  /** Max scan pairs including SOL/USDC. */
  maxScanPairs: number;
  /** Pairs scanned per engine tick (rate-limit rotation). */
  pairsPerScan: number;
  /** Quote mint for alt pairs: USDC or SOL. */
  pairQuoteMint: 'USDC' | 'SOL';
  /** CMC listings to fetch when pairSource=cmc. */
  cmcTopN: number;
  /** Enable Pump.fun bonding curve edge strategy. */
  enablePumpEdge: boolean;
  /** Helius API key for pump launch monitoring. */
  heliusApiKey?: string;

  // ── Execution hardening ──────────────────────────────────────────────────
  /** Enable Jito bundle submission for MEV protection. */
  jitoEnabled: boolean;
  /** Jito tip in lamports added to arb bundles. */
  jitoTipLamports: number;
  /** Minimum net profit in lamports required to execute a trade. */
  minProfitLamports: number;

  // ── Monitoring ───────────────────────────────────────────────────────────
  /** Port for the Express monitoring dashboard. */
  monitorPort: number;
  /** Pino log level (trace | debug | info | warn | error). */
  logLevel: string;
  /** Path to the SQLite trade log database. */
  sqlitePath: string;

  // ── Alerts ───────────────────────────────────────────────────────────────
  /** PnL drop in SOL that triggers a Telegram/Discord alert (negative, e.g. -0.1). */
  alertPnlThresholdSol: number;

  // ── Orca Whirlpool ───────────────────────────────────────────────────────
  /** Comma-separated Orca Whirlpool pool addresses to monitor. */
  orcaPoolAddresses: string[];
  /** Spread in bps that triggers an arb opportunity event. Default 80 (0.8%). */
  spreadThresholdBps: number;
}

const LEGACY_LITE_HOST = 'lite-api.jup.ag';

function env(key: string, fallback: string): string {
  const v = process.env[key]?.trim();
  return v && v.length > 0 ? v : fallback;
}

function warnIfLegacy(url: string, label: string): void {
  if (url.includes(LEGACY_LITE_HOST) || url.includes('quote-api.jup.ag')) {
    console.warn(
      `[config] ${label} uses deprecated Jupiter host — migrate to api.jup.ag (see README)`,
    );
  }
}

export function loadEnv(): BotEnv {
  const jupiterSwapBase = env('JUPITER_SWAP_BASE', 'https://api.jup.ag/swap/v1');
  const jupiterPriceUrl = env('JUPITER_PRICE_URL', 'https://api.jup.ag/price/v3');
  const jupiterTokensBase = env('JUPITER_TOKENS_BASE', 'https://api.jup.ag/tokens/v2');

  warnIfLegacy(jupiterSwapBase, 'JUPITER_SWAP_BASE');
  warnIfLegacy(jupiterPriceUrl, 'JUPITER_PRICE_URL');

  const apiKey = process.env.JUPITER_API_KEY?.trim();
  return {
    jupiterSwapBase: jupiterSwapBase.replace(/\/$/, ''),
    jupiterPriceUrl: jupiterPriceUrl.replace(/\/$/, ''),
    jupiterTokensBase: jupiterTokensBase.replace(/\/$/, ''),
    jupiterApiKey: apiKey && apiKey.length > 0 ? apiKey : undefined,
    rpcUrl: env('SOLANA_RPC_URL', 'https://api.mainnet-beta.solana.com'),
    slippageBps: Number(env('BOT_SLIPPAGE_BPS', '50')),
    minProfitUsd: Number(env('BOT_MIN_PROFIT_USD', '0.25')),
    tradeAmountUi: Number(env('BOT_TRADE_AMOUNT_UI', '1')),
    scanIntervalMs: Number(env('BOT_SCAN_INTERVAL_MS', '2000')),
    paperMode: env('BOT_PAPER_MODE', '1') !== '0',
    walletPublicKey: process.env.BOT_WALLET_PUBKEY?.trim(),
    primaryStrategy: env('BOT_PRIMARY_STRATEGY', 'route_divergence_arb'),
    enableMeanReversion: env('BOT_ENABLE_MEAN_REVERSION', '0') === '1',
    enableTriggerApi: env('BOT_ENABLE_TRIGGER_API', '0') === '1',
    enableRecurringApi: env('BOT_ENABLE_RECURRING_API', '0') === '1',
    cmcApiKey: process.env.CMC_API_KEY?.trim() || process.env.COINMARKETCAP_API_KEY?.trim(),
    pairSource: (env('BOT_PAIR_SOURCE', process.env.CMC_API_KEY ? 'cmc' : 'static') as 'cmc' | 'static'),
    maxScanPairs: Number(env('BOT_MAX_SCAN_PAIRS', '100')),
    pairsPerScan: Number(env('BOT_PAIRS_PER_SCAN', '10')),
    pairQuoteMint: (env('BOT_PAIR_QUOTE_MINT', 'USDC') as 'USDC' | 'SOL'),
    cmcTopN: Number(env('BOT_CMC_TOP_N', '100')),
    enablePumpEdge: env('BOT_ENABLE_PUMP_EDGE', '1') === '1',
    heliusApiKey:
      process.env.HELIUS_API_KEY?.trim() ||
      process.env.SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY?.trim(),

    jitoEnabled: env('JITO_ENABLED', '0') === '1',
    jitoTipLamports: Number(env('JITO_TIP_LAMPORTS', '10000')),
    minProfitLamports: Number(env('MIN_PROFIT_LAMPORTS', '0')),

    monitorPort: Number(env('MONITOR_PORT', '3333')),
    logLevel: env('LOG_LEVEL', 'info'),
    sqlitePath: env('SQLITE_PATH', './trades.db'),

    alertPnlThresholdSol: Number(env('ALERT_PNL_THRESHOLD_SOL', '-0.1')),

    orcaPoolAddresses: (process.env.ORCA_POOL_ADDRESSES ?? '')
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean),
    spreadThresholdBps: Number(env('SPREAD_THRESHOLD_BPS', '80')),
  };
}

/** Well-known mints used by default scanners. */
export const SOL_MINT = 'So11111111111111111111111111111111111111112';
export const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
