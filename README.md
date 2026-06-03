# SolanaArbBot

Cargo workspace for Solana market analysis and arbitrage research.

## Workspace layout

```text
crates/
  accounts
  common
  decoder
  engine
  execution
  graph
  pricing
  risk
  routing
  rpc_client
  stream
```

## Crate responsibilities

- `common`: shared identifiers, market metadata, and normalized market event types.
- `rpc_client`: RPC stream traits, subscription specifications, and RPC error types.
- `stream`: ingestion/event-bus layer built on Tokio and crossbeam channels.
- `decoder`: decoding API boundaries for pools and transactions.
- `accounts`: account cache, loading, and subscription boundaries.
- `pricing`: pricing model and quote request boundaries.
- `graph`: market graph edge and build configuration boundaries.
- `routing`: route and route-search boundaries.
- `risk`: risk check and limit boundaries.
- `execution`: execution planning and transaction build boundaries.
- `engine`: top-level subsystem composition boundary.

## Dependency graph

Dependencies flow from low-level shared types toward higher-level orchestration:

```text
common
├── rpc_client
├── decoder
├── stream ────────────────┐
├── accounts ──────────────┤
│   └── pricing ─ graph ─ routing ─ risk ─ execution
└──────────────────────────┴─────────────── engine
```

Detailed rules:

- `common` has no internal workspace dependencies.
- `rpc_client` depends on `common` for normalized events and subscription identifiers.
- `stream` depends on `common` and `rpc_client`; it does not depend on downstream analysis crates.
- `decoder` depends only on `common`.
- `accounts` depends on `common` and `rpc_client`.
- `pricing` depends on `common` and `accounts`.
- `graph` depends on `common` and `pricing`.
- `routing` depends on `common`, `graph`, and `pricing`.
- `risk` depends on `common` and `routing`.
- `execution` depends on `common`, `rpc_client`, `routing`, and `risk`.
- `engine` depends on all subsystem crates as the top-level composition layer.

This keeps crate dependencies acyclic and prevents low-level ingestion, RPC, or shared types from depending on pricing, routing, execution, or engine code.
