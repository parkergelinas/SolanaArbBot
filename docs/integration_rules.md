# Integration Rules

System-wide constraints for backend, frontend, and shared contracts.

## 1. Single source of truth

- All **live market data** flows through the **stream-api WebSocket broker** (`ws://host:8080/stream`).
- The control-plane API (`control-api` `:3001/ws`) is for **ops only** (config, portfolio, system status).
- The frontend **must not** poll REST for prices, swaps, or candles on the terminal route.

## 2. No frontend mutation of backend state

- Dashboard is **read-only** for stream data.
- Config changes go through `control-api` REST — never via WebSocket from the terminal.
- `marketStore` is derived state only; no optimistic writes to server fields.

## 3. Schema validation

| Boundary | Requirement |
|----------|-------------|
| Rust emit | `serde` + unit tests in `contracts.rs` |
| TypeScript consume | `isWSBatchFrame()` guard before parse |
| CI (future) | JSON Schema validate sample frames |
| Version field | Reject or quarantine `v !== 1` |

When changing types:

1. `shared/contracts/stream/v1.ts`
2. `shared/contracts/stream/schema.v1.json`
3. `apps/stream-api/src/contracts.rs`
4. `apps/dashboard/lib/stream/types.ts`

## 4. Batching consistency

| Layer | Mechanism | Window |
|-------|-----------|--------|
| Server broker | `spawn_batcher` tick | 25–50 ms |
| Client | `requestAnimationFrame` | ≤ 16 ms |
| React | Zustand `applyMessages` once per flush | 1 update/frame |

**Forbidden:** `setState` / Zustand `set` inside `ws.onmessage` per event.

## 5. Store boundaries (frontend)

| Store | Owns |
|-------|------|
| `streamStore` | Connection, latency metrics, batch counters |
| `marketStore` | Prices, swaps, candles, signals, arb opportunities |
| `uiStore` | Selected token, interval, UI toggles |

WS client → `streamStore` → (rAF) → `marketStore`. Components subscribe to stores only.

## 6. Extension points

| Feature | Crate / app |
|---------|-------------|
| Real Solana ingestion | `stream-api/ingestion` (`SwapSource` trait) |
| Execution | `apps/hotpath`, `crates/execution` |
| Jito bundles | `execution::hotpath` cold path |
| Smart money | `crates/signals` → emit `Signal` on stream |

New event types require a **schema version bump** (`v2`), not ad-hoc JSON fields.

## 7. Deployment

| Service | Port | Env |
|---------|------|-----|
| `stream-api` | 8080 | `STREAM_API_PORT`, `STREAM_BATCH_MS` |
| `control-api` | 3001 | `CONTROL_API_PORT` |
| `dashboard` | 3000 | `NEXT_PUBLIC_STREAM_URL`, `NEXT_PUBLIC_WS_URL` |

Production dashboard must set `NEXT_PUBLIC_STREAM_URL=wss://<stream-host>/stream`.
