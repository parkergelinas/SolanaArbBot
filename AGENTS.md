# AGENTS.md

Guidance for AI agents working in **SolanaArbBot**.

## Repository overview

Rust workspace monorepo: arbitrage detection, signal pipeline, paper/live execution controls, and a Next.js dashboard.

| Area | Path | Role |
|------|------|------|
| Config | `crates/config` | `SystemConfig` — single source of truth (TOML + env) |
| Pipeline | `apps/worker` | Paper/backtest runtime orchestrator |
| Control plane | `apps/control-api` | REST + WebSocket hub, bot control, signal buffer |
| Stream | `apps/stream-api` | Trade/signal stream (mirrors control-api via `SIGNAL_HUB_URL`) |
| Intelligence | `backend/intelligence-api` | Whale / smart-money WebSocket |
| Data layer | `backend/data-layer` | Chain ingestion (mock / RPC WS / Geyser) |
| Signal bus | `crates/signal-bus` | Normalized `LiveSignal` buffer + dedup |
| Dashboard | `apps/dashboard` | Next.js UI (Vercel); proxies `/api/*` to control-api |
| Wallet | `crates/wallet` | Key loading (env only), signing, balance guards |
| Domain | `crates/{engine,routing,pricing,risk,...}` | Arb graph, execution, risk |

## Local development

```powershell
.\scripts\dev-services.ps1          # control-api :3001, stream-api :8080, intelligence :8090
cd apps\dashboard; npm run dev      # :3000
```

Set `SOLANA_ARB_RPC__ENDPOINTS` for live chain swaps in data-layer.

## Safety defaults

- `features.enable_live_trading` defaults to **false**
- Live trading requires `SOLANA_ARB_LIVE_CONFIRM=I_UNDERSTAND_REAL_FUNDS`
- Private keys: `SOLANA_ARB_WALLET_KEY` env var only — never key files
- Emergency halt: `touch /tmp/solana_arb_halt` or `SIGUSR1` to worker

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test -p integration-tests
cd apps/dashboard && npm test
```

## Config

Copy `config.example.toml` → `config.toml`. Capital is defined once in `[portfolio].capital_usd`. `routing_min_liquidity` must equal `risk.min_liquidity_usd`.

## Deploy notes

- Dashboard: Vercel (`apps/dashboard`)
- Rust services: Railway/Fly/VPS — see `docs/deploy-control-api.md`
- Env: `CONTROL_API_URL`, `NEXT_PUBLIC_WS_URL`, `SIGNAL_HUB_URL`

## When changing code

1. Match existing crate boundaries — config flows through `SystemConfig`, signals through `signal-bus`.
2. Run `cargo test -p <affected-crate>` before finishing.
3. Do not log key material; wallet `Debug` shows pubkey only.
4. Do not enable live trading in example config.

## Project stage

| Layer | Status |
|-------|--------|
| Worker | **Paper only** — `--mode live` is always rejected at startup |
| Execution | **dry_run default** — `PAPER_MODE=true`, `EXECUTION_LIVE` unset |
| Dashboard | Terminal + overview wired to stream-api, DexScreener, signal hub |
| Live data | Three tiers: **mock/sim** (offline demo) → **data-layer + Jupiter** (degraded) → **full stack** (stream-api + control-api + intel) |

**Live data maturity**

- **Mock / sim**: `DemoBootstrap` fills watchlist, swaps, candles when stream-api is down.
- **Data-layer + Jupiter**: Real quotes via data-layer RPC; stream-api may run in degraded mode.
- **Full stack**: stream-api WS, control-api `/api/live-signals`, DexScreener polls, intelligence-api whales.

**Not implemented yet**: pump.fun ingestion, Yellowstone Geyser, on-chain live trading submission.

## Master wiring validation

1. `cargo build --workspace` — all crates compile with strategy dispatcher.
2. `cargo test --workspace` — unit tests pass (engine dispatcher risk gate, liquidation math).
3. `cargo run --bin worker -- --mode paper` — arb publisher always on; sniper/copy/liquidation/momentum gated by `[strategy]` + section `enabled`.
4. Touch `/tmp/solana_arb_halt` — worker and dispatcher stop accepting new signals.
5. Enable `features.enable_metrics` + `monitoring.enable_prometheus` — scrape `solana_bot_*` metrics.
6. Copy-trading: set `strategy.whale_copy=true`, `copy_trading.enabled=true`, provide Helius key — whale watcher feeds dispatcher bus.
7. Liquidation: `strategy.liquidation=true`, `liquidation.enabled=true` — Kamino scan stub publishes CRITICAL signals.
8. Config: all five sections in `config.example.toml` — `[arbitrage]`, `[sniper]`, `[copy_trading]`, `[liquidation]`, `[momentum]`.
