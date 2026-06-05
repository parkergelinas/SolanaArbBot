# Stream contracts (all agents)

Versioned wire schema for the low-latency trading terminal.

## Layout

| Path | Purpose |
|------|---------|
| `stream/v1.ts` | TypeScript types + guards |
| `stream/schema.v1.json` | JSON Schema for validation / CI |
| `stream/index.ts` | Public exports |

Legacy control-plane types remain in `types.ts` (config, portfolio, health). New market data uses **`stream/v1`**.

## Core types (v1)

- `SwapEvent` — on-chain swap observation
- `TokenPrice` — derived mid price per mint
- `Candle` — OHLCV bucket (`1s` \| `5s` \| `1m`)
- `Signal` — strategy / flow alerts
- `WSMessage` — tagged union (`type` + `payload`)
- `WSBatchFrame` — batched WebSocket frame every 25–50 ms

Every payload includes **`v: 1`**. Breaking changes require `stream/v2.ts` and a parallel Rust module.

## Rust mirror

`apps/stream-api/src/contracts.rs` must match `stream/v1.ts` field-for-field.

## Endpoints

| Service | URL | Schema |
|---------|-----|--------|
| **stream-api** | `ws://localhost:8080/stream` | `WSBatchFrame` v1 |
| control-api | `ws://localhost:3001/ws` | legacy `WsEvent` batch |

## System docs

- [Stream protocol](../../docs/stream_protocol.md)
- [Latency budget](../../docs/latency_budget.md)
- [Integration rules](../../docs/integration_rules.md)

## Sync checklist

1. Edit `shared/contracts/stream/v1.ts` + `schema.v1.json`
2. Mirror in `apps/stream-api/src/contracts.rs`
3. `cargo test -p stream-api`
4. Update dashboard client types when Agent 2 wires the terminal
