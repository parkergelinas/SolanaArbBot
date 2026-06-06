# Solana Arb Bot (TypeScript)

Standalone Jupiter scanner and paper/live execution runner with a **4-layer architecture**.

## Jupiter API (current — not lite-api)

As of January 2026, `lite-api.jup.ag` and `quote-api.jup.ag` are deprecated. This bot uses:

| Service | Default URL | Purpose |
|---------|-------------|---------|
| Swap API v1 | `https://api.jup.ag/swap/v1` | `/quote`, `/swap` |
| Price API v3 | `https://api.jup.ag/price/v3` | USD prices |
| Tokens API v2 | `https://api.jup.ag/tokens/v2` | Verified/strict token universe |
| Trigger API v1 | `https://api.jup.ag/trigger/v1` | Stub — TP/SL/breakout (not wired) |
| Recurring API v1 | `https://api.jup.ag/recurring/v1` | Stub — DCA/treasury (not wired) |

Optional: set `JUPITER_API_KEY` for authenticated requests.

## 4-layer architecture

```
MarketData  →  Signal  →  Risk  →  Execution
     │            │         │          │
     │            │         │          └─ live-executor (retry, simulate, journal)
     │            │         └─ limits (per-strategy, cooldowns, inventory)
     │            └─ Strategy.evaluate(marketState) → TradeDecision
     └─ Price v3 + Tokens v2 → MarketState (universe, quality, prices)
```

### Layer modules

| Layer | Path | Responsibility |
|-------|------|----------------|
| **MarketData** | `src/market-data/` | Curated universe, USD prices, token quality |
| **Signal** | `src/signals/` | Pluggable strategies, registry, trade decisions |
| **Risk** | `src/risk/` | Session loss, cooldowns, route-family failures |
| **Execution** | `src/execution/` | Paper/live swap, route metadata logging |

### Engine orchestration (`src/app/engine.ts`)

```text
MarketData.buildState()
  → ScannableStrategy.scan()   # fetch quotes per strategy
  → StrategyRegistry.best()    # pick highest-edge decision
  → Risk.checkStrategyRisk()
  → DynamicSizing
  → Execution.submit()
```

## Strategies

| Strategy | ID | Status |
|----------|-----|--------|
| **Route divergence arb** | `route_divergence_arb` | **Primary** — restricted vs unrestricted routes, staleness + fee survival gates |
| Round-trip quote arb | `round_trip_quote_arb` | Legacy baseline |
| Pairs mean reversion | `pairs_mean_reversion` | Scaffold — rolling z-score (disabled by default) |
| Trigger automation | — | Client stub only |
| Recurring DCA | — | Client stub only |

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
| `BOT_PRIMARY_STRATEGY` | `route_divergence_arb` | Active strategy |
| `BOT_ENABLE_MEAN_REVERSION` | `0` | Enable mean-reversion scaffold |
| `BOT_ENABLE_TRIGGER_API` | `0` | Enable trigger stub (not wired) |
| `BOT_ENABLE_RECURRING_API` | `0` | Enable recurring stub (not wired) |
| `BOT_PAPER_MODE` | `1` | Paper trading (no on-chain send) |
| `BOT_MIN_PROFIT_USD` | `0.25` | Minimum net edge |
| `BOT_TRADE_AMOUNT_UI` | `1` | Base SOL trade size |
| `BOT_SCAN_INTERVAL_MS` | `2000` | Scan interval |
| `BOT_MAX_ITERATIONS` | `0` | Stop after N scans (0 = infinite) |

## Adding strategies

1. Implement `Strategy` (or `ScannableStrategy` if quotes are needed) in `src/signals/`.
2. Register in `BotEngine` constructor.
3. Add per-strategy risk rules in `src/risk/limits.ts` if needed.

Planned: full mean-reversion execution, trigger-based entries, recurring DCA.
