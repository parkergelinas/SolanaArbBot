# Intelligence Terminal

Minimal real-time Solana intelligence UI consuming the intelligence-api WebSocket stream.

## Stack

- **Frontend**: Next.js 14 on port 3010 (`frontend/`)
- **Backend**: `intelligence-api` on port 8090 (`/intelligence` WS)
- **Data**: `data-layer` pipeline (mock or Yellowstone)

## Components

| Component | Purpose |
|-----------|---------|
| `WhaleFeed` | Live whale alerts |
| `WalletRail` | Smart money wallet snapshots |
| `TxTape` | Recent swap tape |
| `StreamBootstrap` | WS connection bootstrap |

## Run

```bash
# Terminal 1
cargo run -p intelligence-api

# Terminal 2
cd frontend && npm install && npm run dev
```

Set `NEXT_PUBLIC_INTELLIGENCE_URL=ws://localhost:8090/intelligence`.

## Full pipeline

```
data-layer → intelligence-api → frontend (display)
                    ↓
              alpha-engine → execution-engine (trading)
arb-engine ─────────┘
```
