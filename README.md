
# SolanaArbBot

[![CI](https://github.com/parkergelinas/SolanaArbBot/actions/workflows/ci.yml/badge.svg)](https://github.com/parkergelinas/SolanaArbBot/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

SolanaArbBot is a mono-repository for high-performance arbitrage detection and execution across Solana markets. The repo contains Rust engines, services, and auxiliary frontend and tooling used for development, backtesting, and monitoring.

Core goals:
- Detect cross-market arbitrage opportunities quickly
- Construct and submit atomic transactions to capture spreads
- Provide tools for backtesting, monitoring, and analysis

Repository layout (top-level)
- `apps/` — runnable services and apps (backtester, **bot**, control-api, dashboard, hotpath, stream-api, worker)
- `backend/` — larger backend engines and services (alpha-engine, arb-engine, execution-engine, etc.)
- `crates/` — reusable Rust crates used across the workspace (accounts, common, pricing, routing, rpc_client, etc.)
- `frontend/` & `dashboard/` — Next.js frontend apps and UI components
- `docs/` — design docs, architecture notes, and guides
- `infrastructure/` — deployment and monitoring manifests
- `scripts/` — helper scripts (changelog generation, security checks, dev helpers)
- `alertmanager/`, `grafana/`, `prometheus/` — observability configs and dashboards

Key files
- `config.example.toml` — example runtime configuration
- `AGENTS.md` — guidance for AI agents and contributor tooling
- `MONOREPO.md` — monorepo conventions and development workflow

Prerequisites
- Rust toolchain (tested with `rustc`/`cargo` 1.83+)
- Node.js 22+ (used by Next.js frontends; `pnpm`/`npm`/`yarn` supported)
- Python 3.12+ (utilities and scripts in `scripts/`)

Build and test (overview)
- Build the Rust workspace:

```bash
cargo build --workspace --release
```

- Run Rust tests across the workspace:

```bash
cargo test --workspace
```

- Frontend (example for `frontend/` or `dashboard/`):

```bash
cd frontend
pnpm install
pnpm build
pnpm test
```

- TypeScript Jupiter bot (`apps/bot/` — uses `api.jup.ag`, not deprecated `lite-api.jup.ag`):

```bash
cd apps/bot
npm install
npm test
BOT_PAPER_MODE=1 BOT_MAX_ITERATIONS=3 npm run dev
```

Configuration and running
- Copy `config.example.toml` to a working config and edit as needed.
- Free live data: set `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY` (from [helius.dev](https://helius.dev) free tier).
- Capital is defined once in `[portfolio].capital_usd`. `risk.min_liquidity_usd` must equal `pipeline.routing_min_liquidity`.
- Each service in `apps/` or `backend/` contains its own README or run instructions — consult the crate or package folder for details.

## Security

Never store private keys in config files. Inject via env var `SOLANA_ARB_WALLET_KEY` (base58 encoded). Key files are never read from disk.

## Emergency Stop

To halt immediately: `touch /tmp/solana_arb_halt` or send `SIGUSR1` to the worker process.

Never run with `enable_live_trading=true` without also setting `SOLANA_ARB_LIVE_CONFIRM=I_UNDERSTAND_REAL_FUNDS`.

Development notes
- This repo is organized as a Rust workspace with multiple service crates and frontend apps. Use `cargo` to build crates and the usual Node toolchain for frontends.
- Backtesting data and examples are stored under `data/` (e.g., `data/backtest_results.json`).

Contributing
- See [AGENTS.md](AGENTS.md) for repository policies, agent guidance, and contributor workflow.

Documentation
- Design documents, architecture notes, and developer guides are in the `docs/` directory.

License
- MIT


