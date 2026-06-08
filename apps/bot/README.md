# Solana Arb Bot (TypeScript)

Standalone Jupiter quote scanner and paper/live execution runner. It is separate
from the Rust worker pipeline and is useful for fast strategy experiments,
Jupiter API validation, and paper sessions that journal every decision.

## Current Jupiter APIs

`lite-api.jup.ag` and `quote-api.jup.ag` are deprecated for this bot. Defaults
come from `src/config/env.ts`:

| Service | Default URL | Purpose |
| --- | --- | --- |
| Swap API v1 | `https://api.jup.ag/swap/v1` | `/quote` and `/swap` |
| Price API v3 | `https://api.jup.ag/price/v3` | USD prices |
| Tokens API v2 | `https://api.jup.ag/tokens/v2` | Verified/strict token universe |

Set `JUPITER_API_KEY` when running repeated scans. Do not commit real API keys or
wallet keys; keep them in a local `.env` file or your shell environment.

## Architecture

```text
Market data -> Scannable strategies -> Strategy registry -> Risk -> Sizing -> Execution
```

| Layer | Main paths | Notes |
| --- | --- | --- |
| Market data | `src/market-data/`, `src/market/` | Builds `MarketState`, pair universes, prices, and token quality. |
| Signals | `src/signals/`, `src/strategy/` | Strategies fetch quotes and emit `TradeDecision` objects. |
| Risk | `src/risk/` | Session loss, cooldown, inventory, and route-family limits. |
| Sizing | `src/sizing/dynamic.ts` | Caps trade size using spread, liquidity, volatility, route quality, failures, and fee state. |
| Execution | `src/execution/`, `src/mev/` | Paper execution, live swap hardening, dead-man switch, SQLite persistence, optional Jito submission. |
| Monitoring | `src/monitoring/`, `src/analytics/` | Express endpoints and journal/session analytics. |

`BotEngine` (`src/app/engine.ts`) orchestrates the loop:

1. Refresh the pair registry once, then build a `MarketState` each scan.
2. Run every registered `ScannableStrategy`.
3. Pick the configured strategy result or the best registered decision.
4. Apply risk gates and dynamic sizing.
5. Execute paper decisions through `LiveExecutor`'s paper branch or live
   decisions through `HardenedExecutor`.
6. Update journal, stats, risk state, and monitoring data.

## Strategies

| Strategy | ID | Enablement | What it does |
| --- | --- | --- | --- |
| Route divergence arb | `route_divergence_arb` | Default when `BOT_ENABLE_CROSS_DEX_ARB=0` | Compares restricted vs unrestricted Jupiter routes and gates on surviving edge. |
| Round-trip quote arb | `round_trip_quote_arb` | Registered with route divergence | Legacy baseline strategy. |
| Cross-DEX arb | `cross_dex_arb` | `BOT_ENABLE_CROSS_DEX_ARB=1` | Compares Raydium-only vs Orca/Whirlpool-only Jupiter quotes for the same pair. |
| Pump edge | `pump_edge` | `BOT_ENABLE_PUMP_EDGE=1` | Watches configured/new mints for pump.fun-style bonding-curve edges. |
| Mean reversion | `pairs_mean_reversion` | `BOT_ENABLE_MEAN_REVERSION=1` | Rolling z-score scaffold, disabled by default. |

Cross-DEX mode scans static core pairs from `CROSS_DEX_CORE_PAIRS` and can prepend
DexScreener-discovered Solana memecoin pairs from `PumpTokenRegistry` when
`BOT_ENABLE_PUMP_SPREADS=1`. The scanner fetches two DEX-restricted quotes per
pair, subtracts a 30 bps fee floor, and rejects decisions below the configured
spread/profit gates.

Current live-execution caution: cross-DEX decisions carry metadata noting that a
two-transaction atomic bundle is required for true venue arbitrage. The live path
still submits the selected `TradeDecision` through the hardened Jupiter swap
executor, so validate paper/live-quote behavior and executor semantics before
using real funds.

## Quick start

```bash
cd apps/bot
npm install
npm test
BOT_PAPER_MODE=1 BOT_MAX_ITERATIONS=3 npm run dev
```

`npm run dev` uses `tsx --env-file=.env src/index.ts`, so values in
`apps/bot/.env` are loaded automatically. `BOT_MAX_ITERATIONS=0` or an unset
value runs indefinitely.

### Paper cross-DEX example

```bash
cat > .env <<'EOF'
BOT_PAPER_MODE=1
BOT_LIVE_QUOTES=1
BOT_PRIMARY_STRATEGY=cross_dex_arb
BOT_ENABLE_CROSS_DEX_ARB=1
BOT_ENABLE_PUMP_SPREADS=1
BOT_ENABLE_PUMP_EDGE=0
BOT_TRADE_AMOUNT_UI=0.9
BOT_MIN_PROFIT_USD=0.01
BOT_SCAN_INTERVAL_MS=5000
BOT_PAIRS_PER_SCAN=8
JUPITER_API_KEY=replace-me
SQLITE_PATH=./trades-cross-dex.db
EOF

BOT_MAX_ITERATIONS=3 npm run dev
```

This mode uses real Jupiter quotes but keeps execution simulated when
`BOT_PAPER_MODE=1`.

## Runtime modes and safety

| Mode | Required settings | Behavior |
| --- | --- | --- |
| Paper (default) | `BOT_PAPER_MODE=1` | No on-chain send. Successful paper executions journal `realizedProfitUsd = netProfitUsd * 0.85`. |
| Live quotes | `BOT_PAPER_MODE=1`, `BOT_LIVE_QUOTES=1` | Uses real quote/price polling for validation while retaining paper execution. |
| Live execution | `BOT_PAPER_MODE=0`, `SOLANA_ARB_WALLET_KEY` | Uses `HardenedExecutor`: fresh quote, priority-fee estimate, simulation, retries, SQLite persistence, and dead-man switch. |
| Optional Jito | `JITO_ENABLED=1` | Attempts Jito bundle submission with `JITO_TIP_LAMPORTS`; falls back to standard RPC if bundle submission fails. |

Live execution only loads wallet material from `SOLANA_ARB_WALLET_KEY`
(base58-encoded secret key or JSON byte array). `BOT_WALLET_PUBKEY` is for
monitoring balance and unsigned Jupiter route metadata; it is not enough to sign
live transactions.

The dead-man switch halts the engine after three consecutive hardened-executor
failures. `SIGINT`/`SIGTERM` trigger graceful shutdown and wait for in-flight
trades before closing SQLite.

## Environment reference

### Core runtime

| Variable | Default | Description |
| --- | --- | --- |
| `SOLANA_RPC_URL` | `https://api.mainnet-beta.solana.com` | RPC used for balances, priority fees, simulation, and sends. |
| `BOT_PAPER_MODE` | `1` | `1` avoids on-chain sends; `0` enables live executor. |
| `BOT_LIVE_QUOTES` | `0` | Poll real Jupiter prices/quotes while still paper trading. |
| `BOT_PRIMARY_STRATEGY` | `route_divergence_arb` | Preferred strategy ID. |
| `BOT_MIN_PROFIT_USD` | `0.25` | Minimum net edge for decisions. |
| `BOT_TRADE_AMOUNT_UI` | `1` | Base amount in UI units for strategy scans. |
| `BOT_SLIPPAGE_BPS` | `50` | Slippage passed to Jupiter quote/swap calls. |
| `BOT_SCAN_INTERVAL_MS` | `2000` | Delay between scan loops. |
| `BOT_MAX_ITERATIONS` | `0` | `src/index.ts` loop cap; `0`/unset means unlimited. |

### Strategy selection and pair universe

| Variable | Default | Description |
| --- | --- | --- |
| `BOT_ENABLE_CROSS_DEX_ARB` | `0` | Replaces route-divergence/round-trip scanners with cross-DEX scanning. |
| `BOT_CROSS_DEX_SPREAD_BPS` | `35` | Minimum spread for core cross-DEX pairs. |
| `BOT_PUMP_SPREAD_BPS` | `80` | Minimum spread for pump/memecoin cross-DEX pairs. |
| `BOT_ENABLE_PUMP_SPREADS` | `1` | Enable DexScreener memecoin discovery for cross-DEX scans. |
| `BOT_ENABLE_PUMP_EDGE` | `0` | Enable the separate pump bonding-curve strategy. |
| `BOT_ENABLE_MEAN_REVERSION` | `0` | Enable the mean-reversion scaffold. |
| `BOT_ENABLE_TRIGGER_API` | `0` | Parsed for Jupiter Trigger API experiments; not wired into execution. |
| `BOT_ENABLE_RECURRING_API` | `0` | Parsed for Jupiter Recurring API experiments; not wired into execution. |
| `BOT_PAIR_SOURCE` | `cmc` if `CMC_API_KEY` is set, else `static` | Pair registry source. |
| `BOT_MAX_SCAN_PAIRS` | `100` | Maximum pairs in the registry. |
| `BOT_PAIRS_PER_SCAN` | `10` | Pair batch size per loop; cross-DEX makes two Jupiter quote calls per pair. |
| `BOT_PAIR_QUOTE_MINT` | `USDC` | Quote mint for alternative pairs (`USDC` or `SOL`). |
| `BOT_CMC_TOP_N` | `100` | CoinMarketCap listings fetched when using CMC. |
| `BOT_MIN_DIVERGENCE_BPS` | `5` | Route-divergence minimum raw divergence. |
| `BOT_MIN_SURVIVING_EDGE_BPS` | `3` | Route-divergence minimum edge after costs. |

### External services, execution, and monitoring

| Variable | Default | Description |
| --- | --- | --- |
| `JUPITER_SWAP_BASE` | `https://api.jup.ag/swap/v1` | Jupiter Swap API base. |
| `JUPITER_PRICE_URL` | `https://api.jup.ag/price/v3` | Jupiter Price API base. |
| `JUPITER_TOKENS_BASE` | `https://api.jup.ag/tokens/v2` | Jupiter Tokens API base. |
| `JUPITER_API_KEY` | unset | Optional authenticated Jupiter requests. |
| `CMC_API_KEY` / `COINMARKETCAP_API_KEY` | unset | Enables CMC pair registry. |
| `HELIUS_API_KEY` / `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY` | unset | Optional pump launch monitoring key. |
| `SOLANA_ARB_WALLET_KEY` | unset | Live signing key; never store in committed files. |
| `BOT_WALLET_PUBKEY` | unset | Public key used by monitoring and unsigned route metadata. |
| `JITO_ENABLED` | `0` | Attempt Jito bundle submission in live mode. |
| `JITO_TIP_LAMPORTS` | `10000` | Tip added to Jito transactions. |
| `MIN_PROFIT_LAMPORTS` | `0` | Fee-vs-profit guard for live execution. |
| `MONITOR_PORT` | `3333` | Express monitoring server port. |
| `LOG_LEVEL` | `info` | Pino log level. |
| `SQLITE_PATH` | `./trades.db` | SQLite trade log path. |
| `ALERT_PNL_THRESHOLD_SOL` | `-0.1` | PnL alert threshold converted using live SOL price. |
| `TELEGRAM_BOT_TOKEN` / `TELEGRAM_CHAT_ID` | unset | Optional Telegram alert delivery. |
| `DISCORD_WEBHOOK_URL` | unset | Optional Discord alert delivery. |
| `ORCA_POOL_ADDRESSES` | built-in SOL/USDC pools | Comma-separated Orca Whirlpool pools for event detection. Unknown pools are skipped unless metadata is added in `BotEngine`. |
| `SPREAD_THRESHOLD_BPS` | `80` | Event-driven opportunity detector threshold. |

## Monitoring API

The bot starts an Express server when `runLoop()` starts.

| Endpoint | Description |
| --- | --- |
| `GET /health` | `200` with `status: "running"` or `503` with `status: "halted"`. |
| `GET /status` | Uptime, optional wallet balance, trade counts, and total PnL. |
| `GET /trades?limit=50` | Recent SQLite trades, capped at 200. |
| `GET /journal?type=execution&limit=100` | In-memory journal events, capped at 500. |
| `GET /scan-stats` | Engine scan/action/execution/rejection stats. |
| `GET /analytics` | Session hit rate, rejection breakdown, PnL distribution, and per-pair stats. |

Example:

```bash
curl http://localhost:3333/health
curl 'http://localhost:3333/journal?type=rejection&limit=25'
```

## Common pitfalls

- `BOT_ENABLE_CROSS_DEX_ARB=1` disables the route-divergence and round-trip
  scanners to avoid competing for the same Jupiter quota.
- Cross-DEX scans are quota-heavy: `BOT_PAIRS_PER_SCAN=8` means 16 quote calls
  per scan before price polling or other strategies.
- `BOT_LIVE_QUOTES=1` starts Jupiter price polling even when paper trading.
- `BOT_WALLET_PUBKEY` is not a signing key. Live execution needs
  `SOLANA_ARB_WALLET_KEY`.
- DexScreener pump-pair discovery is best-effort and silently keeps the stale
  cache on network errors.
- Unknown `ORCA_POOL_ADDRESSES` are skipped until their token metadata is added
  to `WELL_KNOWN_ORCA_POOLS` in `src/app/engine.ts`.

## Adding strategies

1. Implement `Strategy` or `ScannableStrategy` in `src/signals/`.
2. Register it in the `BotEngine` constructor.
3. Add risk handling in `src/risk/limits.ts` if the strategy needs custom
   cooldowns, session limits, or route-family rules.
4. Add tests under `apps/bot/tests/` and document any new environment variables
   in this README.
