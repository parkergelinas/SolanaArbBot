# Solana Arb Bot (TypeScript)

Standalone Jupiter quote-arbitrage scanner and paper/live execution runner.

## Jupiter API (current — not lite-api)

As of January 2026, `lite-api.jup.ag` and `quote-api.jup.ag` are deprecated. This bot uses:

| Service | Default URL | Purpose |
|---------|-------------|---------|
| Swap API v1 | `https://api.jup.ag/swap/v1` | `/quote`, `/swap` |
| Price API v3 | `https://api.jup.ag/price/v3` | USD prices |
| Tokens API v2 | `https://api.jup.ag/tokens/v2` | Verified/strict token universe |

Optional: set `JUPITER_API_KEY` for authenticated requests.

## Architecture

```
src/config/env.ts          → api.jup.ag defaults
src/jupiter/client.ts      → Swap v1 + Price v3 + Tokens v2
src/market/data-layer.ts   → MarketState (prices, quality, universe)
src/strategy/
  types.ts                 → Strategy.evaluate(marketState) → TradeDecision
  arb-scanner.ts           → forward + reverse quote capture
  scorer.ts                → USD-normalized PnL (no unit mixing)
  round-trip-arb.ts        → first strategy implementation
  route-quality.ts         → hops, impact, freshness gates
src/app/engine.ts          → scan loop + dynamic sizing + risk
src/backtest/backtester.ts → staleness, slippage, missed fills
src/execution/live-executor.ts → simulate, retry, route logging
src/analytics/metrics.ts   → hit rate, drift, rejection distribution
```

## Quick start

```bash
cd apps/bot
npm install
npm test
BOT_PAPER_MODE=1 BOT_MAX_ITERATIONS=3 npm run dev
```

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `JUPITER_SWAP_BASE` | `https://api.jup.ag/swap/v1` | Swap API base |
| `JUPITER_PRICE_URL` | `https://api.jup.ag/price/v3` | Price API |
| `JUPITER_TOKENS_BASE` | `https://api.jup.ag/tokens/v2` | Tokens API |
| `JUPITER_API_KEY` | — | Optional API key |
| `BOT_PAPER_MODE` | `1` | Paper trading (no on-chain send) |
| `BOT_MIN_PROFIT_USD` | `0.25` | Minimum net edge |
| `BOT_TRADE_AMOUNT_UI` | `1` | Base SOL trade size |
| `BOT_SCAN_INTERVAL_MS` | `2000` | Scan interval |
| `BOT_MAX_ITERATIONS` | `0` | Stop after N scans (0 = infinite) |

## Adding strategies

Implement `Strategy` in `src/strategy/types.ts` and register in `BotEngine`.

Planned extensions: route divergence arb, mean reversion, momentum, trigger-based entries.
