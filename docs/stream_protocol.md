# Stream Protocol Specification (v1)

Canonical wire format for the low-latency Solana trading terminal.

## Transport

| Property | Value |
|----------|-------|
| Protocol | WebSocket (text JSON today) |
| Endpoint | `ws://<host>:8080/stream` |
| Schema version | `v: 1` |
| Service | `apps/stream-api` |

## Message envelope

Clients receive **only batched frames** — never single domain events on the hot path.

```json
{
  "v": 1,
  "type": "batch",
  "seq": 42,
  "ts_ms": 1717500000123,
  "messages": [
    { "type": "swap", "payload": { ... } },
    { "type": "token_price", "payload": { ... } }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `v` | `u32` | Contract version — must be `1` |
| `type` | `"batch"` | Frame discriminator |
| `seq` | `u64` | Monotonic batch sequence |
| `ts_ms` | `u64` | Server flush timestamp (Unix ms) |
| `messages` | `WSMessage[]` | Coalesced events |

## Domain messages (`WSMessage`)

Tagged union: `{ "type": "<kind>", "payload": { ... } }`

| `type` | Payload | Emitted when |
|--------|---------|--------------|
| `swap` | `SwapEvent` | On-chain swap observed |
| `token_price` | `TokenPrice` | Derived mid price update |
| `candle` | `Candle` | OHLCV bucket closed/updated |
| `signal` | `Signal` | Strategy / flow alert |

All payloads include `"v": 1`.

## Batching rules

| Rule | Value |
|------|-------|
| Server flush window | **25–50 ms** (default **33 ms**) |
| Env override | `STREAM_BATCH_MS` |
| Client coalesce | `requestAnimationFrame` (~16 ms, max 60 FPS) |
| Per-event WS send | **Forbidden** on production path |

Server pipeline:

```
ingestion (crossbeam) → market engine → broker buffer → flush tick → broadcast
```

Client pipeline:

```
onmessage → parse batch → inbox buffer → rAF flush → Zustand marketStore
```

## Backpressure

### Server

1. **Ingestion → engine**: unbounded crossbeam channel; mock source rate-limited by `MOCK_SWAP_INTERVAL_MS`.
2. **Engine → broker**: unbounded crossbeam; engine runs on dedicated thread.
3. **Broker → WS clients**: `tokio::sync::broadcast` capacity **512** frames.
4. **Lagged clients**: `RecvError::Lagged(n)` — drop `n` batches, log warning, client resyncs from next batch.

Future: bounded channels with `try_send` + drop-oldest policy for hot path.

### Client

1. Inbox grows between rAF ticks; multiple server batches coalesce into one render.
2. Ring buffers in `marketStore`: swaps (500), signals (80), candle history (120 per key).
3. Virtualized swap feed renders only visible rows.

## Future binary encoding

JSON is the v1 default for debuggability. v2 may add:

| Option | Notes |
|--------|-------|
| **MessagePack** | Same schema, smaller frames |
| **FlatBuffers / Cap'n Proto** | Zero-copy decode on frontend via WASM |
| **Negotiation** | `Sec-WebSocket-Protocol: solarb-stream-v1-json` → `v2-msgpack` |

Binary frames will retain `v`, `seq`, `ts_ms` header; payloads versioned separately.

## Schema sources

| Layer | Path |
|-------|------|
| TypeScript | `shared/contracts/stream/v1.ts` |
| JSON Schema | `shared/contracts/stream/schema.v1.json` |
| Rust | `apps/stream-api/src/contracts.rs` |

## Heartbeat / latency

Every batch carries `ts_ms` for one-way latency estimation:

```
ws_latency_ms ≈ Date.now() - frame.ts_ms
```

Optional connect batch includes a `signal` with `detail: "connected"`.
