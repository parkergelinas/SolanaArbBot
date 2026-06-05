# Architecture

| Crate / App | Role | Status |
|-------------|------|--------|
| `crates/config` | `SystemConfig` loader (TOML + env), validation, live-trading guards | Canonical |
| `crates/common` | Shared types (`Pubkey`, errors) | Canonical |
| `crates/events` | In-process `EventBus` | Canonical |
| `crates/signals` | Signal engine + feature store | Canonical |
| `crates/signal-bus` | Normalized live signals, dedup, JSONL replay | Canonical |
| `crates/scalper` | Trade filter chain + paper fills | Canonical |
| `crates/wallet` | Env-only keypair, signing, RPC balance | Canonical |
| `crates/engine` | Arb detection core | Canonical |
| `crates/routing` | Route DFS / scoring | Canonical |
| `crates/pricing` | Pool pricing | Canonical |
| `crates/risk` | Route-level risk | Canonical |
| `crates/portfolio` | Portfolio risk state machine | Canonical |
| `crates/execution` | Tx building / simulation | Canonical |
| `crates/orchestrator` | Run-mode gate + lifecycle | Canonical |
| `crates/autonomous` | Strategy runtime controller | Canonical |
| `crates/backtester` | Historical replay + optimization | Canonical |
| `crates/stream` | Re-export wrapper | Wrapper |
| `crates/rpc_client` | Re-export wrapper | Wrapper |
| `crates/risk_engine` | Legacy risk enforcer | Wrapper |
| `apps/worker` | Paper/backtest pipeline binary | Build OK |
| `apps/control-api` | Control plane + signal hub | Build OK |
| `apps/stream-api` | WS stream + hub mirror | Build OK |
| `apps/hotpath` | Low-latency engine binary | Build OK |
| `apps/backtester` | CLI backtest runner | Build OK |
| `backend/data-layer` | Chain ingestion adapters | Build OK |
| `backend/intelligence-api` | Whale alerts WS | Build OK |
| `backend/execution-engine` | Signal → Jupiter execution router | Build OK |
| `backend/arb-engine` | Spread detection + WS | Build OK |
| `backend/alpha-engine` | Alpha signal research | Build OK |
| `apps/dashboard` | Next.js operator UI | Build OK |
| `tests` (`integration-tests`) | Pipeline smoke scaffold | Scaffold |

## Data flow

```
data-layer (mock | RPC WS | Geyser)
  → signal-bus (LiveSignal)
    → control-api (:3001)  /api/live-signals, /ws
    → stream-api (:8080)   mirrors via SIGNAL_HUB_URL
  → intelligence-api → whale alerts → signal-bus

worker (paper): EventBus → signals → scalper
autonomous: strategy controller → execution paths
dashboard: control-api REST/WS proxy
```
