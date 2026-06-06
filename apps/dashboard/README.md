# Solana Arb Dashboard

Next.js trading terminal and control-plane UI. Deploy root: `apps/dashboard` (see repo `MONOREPO.md`).

## Data sources (production)

| Layer | Source | Purpose |
|-------|--------|---------|
| **Prices** | DexScreener API | USD quotes, 24h change, volume (authoritative for display) |
| **Market stream** | `stream-api` WS (`NEXT_PUBLIC_STREAM_URL`) | Live swaps, signals, cross-DEX arb |
| **Control plane** | `control-api` WS/REST | Bot status, portfolio, config |
| **Wallet** | Solana RPC (devnet/mainnet) | On-chain balances |

Demo/mock data is **disabled by default**. Set `NEXT_PUBLIC_TERMINAL_DEMO=1` only for offline UI development.

## Devnet local setup

### 1. Environment

```bash
cd apps/dashboard
cp .env.example .env.local
```

Ensure `.env.local` has:

- `NEXT_PUBLIC_SOLANA_NETWORK=devnet`
- `NEXT_PUBLIC_TERMINAL_DEMO=0`
- `NEXT_PUBLIC_STREAM_URL=ws://localhost:8080/stream`

### 2. Backend services (separate terminals)

```bash
# Stream engine (port 8080)
cargo run -p stream-api

# Control API (port 3001)
cargo run -p control-api
```

### 3. Frontend

```bash
npm install
npm run dev
```

Open [http://localhost:3000/terminal](http://localhost:3000/terminal)

### 4. Verify real data

- Top nav shows **LIVE** or **OFFLINE** (not DEMO unless `TERMINAL_DEMO=1`)
- SOL price matches DexScreener (~market rate, not hardcoded)
- Watchlist and navbar show the same price
- Orderbook fills when stream-api sends cross-DEX swaps; otherwise shows connect message
- Wallet: switch to Devnet in the nav cluster toggle

## Scripts

```bash
npm run dev      # port 3000
npm run build
npm run test
```

## Vercel

Set root directory to `apps/dashboard` and configure the same `NEXT_PUBLIC_*` variables for your deployed backends.
