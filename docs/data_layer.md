# Solana Real-Time Data Layer

Production-grade streaming pipeline for low-latency trading intelligence.

## Architecture

```
Yellowstone gRPC (or MockAdapter)
        ↓
   RawUpdate { slot, signature, logs, accounts, timestamp }
        ↓
   Instruction Parser (Raydium / Orca / Jupiter log decoding)
        ↓
   Normalization → SwapEvent { signature, wallet, token, amount_sol, dex, timestamp }
        ↓
   Enrichment (wallet labels + token metadata via DashMap)
        ↓
   Event Router (crossbeam fan-out)
        ↓
   intelligence-api → whale detector + wallet tracker + WS broker
        ↓
   frontend/ (ws://localhost:8090/intelligence)
```

## Latency budget

| Stage            | Target   |
|------------------|----------|
| Ingest → parse   | < 5 ms   |
| Normalize/enrich | < 10 ms  |
| Detection        | < 5 ms   |
| WS batch flush   | 25–50 ms |
| Frontend render  | < 33 ms  |
| **Perceived E2E**| **< 150 ms** |

## Crates

| Path | Role |
|------|------|
| `backend/data-layer` | Ingest, parse, normalize, enrich, route |
| `backend/intelligence-api` | Whale/smart detection, wallet tracker, axum WS |
| `shared/contracts/intelligence` | Typed wire contracts |
| `frontend/` | Minimal signal-first UI |

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `YELLOWSTONE_ENDPOINT` | — | Geyser gRPC URL. Unset → mock mode |
| `MOCK_INGEST_INTERVAL_MS` | `40` | Mock swap cadence |
| `WHALE_THRESHOLD_SOL` | `10.0` | Whale alert threshold |
| `INTELLIGENCE_PORT` | `8090` | HTTP/WS listen port |
| `INTELLIGENCE_BATCH_MS` | `33` | WS batch interval (25–50) |
| `NEXT_PUBLIC_INTELLIGENCE_URL` | `ws://localhost:8090/intelligence` | Frontend WS URL |

## Run — mock mode (local)

```bash
# Terminal 1 — intelligence API (includes data-layer pipeline)
cargo run -p intelligence-api

# Terminal 2 — minimal frontend
cd frontend && npm install && npm run dev
```

Open http://localhost:3010

## Run — Yellowstone mode

```bash
# Build with gRPC feature (proto vendoring in follow-up)
cargo build -p data-layer --features yellowstone

export YELLOWSTONE_ENDPOINT=http://127.0.0.1:10000
cargo run -p intelligence-api
```

## Ingestion adapters

| Adapter | Status |
|---------|--------|
| `MockAdapter` | ✅ High-frequency local stream |
| `GeyserAdapter` | 🔧 Stub (feature `yellowstone`) |
| `RpcAdapter` | 🔧 Helius stub |

## Contracts

See `shared/contracts/intelligence/v1.ts` for `RawUpdate`, `SwapEvent`, `WhaleAlert`, `WalletSnapshot`, `IntelligenceBatch`.
