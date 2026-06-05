# Monorepo layout — Solana low-latency trading terminal

Stream-first architecture: **Rust stream engine → batched WebSocket → Next.js terminal**.

```
SolanaArbBot/
├── apps/
│   ├── stream-api/      # Agent 1: Market stream engine — ingestion, DashMap state, ws://:8080/stream
│   ├── control-api/     # Control plane — config, portfolio, legacy /ws on :3001
│   ├── dashboard/       # Agent 2: Next.js trading terminal (external store, no per-event React state)
│   ├── worker/          # Background paper/live worker (feeds signals into control-api later)
│   └── hotpath/         # Ultra-low-latency execution binary (separate process)
├── shared/
│   └── contracts/       # Agent 3: WS types, JSON schema, sync checklist
├── crates/              # Rust domain pipeline (per docs/system_contract.md)
└── docs/
    └── system_contract.md
```

## Agent responsibilities

| Agent | Path | Deliverable |
|-------|------|-------------|
| **1 — Rust backend** | `apps/stream-api` | Mock ingestion, market engine, `WSMessage` broker on `/stream` |
| **1b — Control plane** | `apps/control-api` | REST + legacy batched `/ws` for ops UI |
| **2 — Frontend** | `apps/dashboard` | `lib/stream-store.ts`, `useSyncExternalStore` hooks |
| **3 — Contracts** | `shared/contracts` | `types.ts`, `ws-events.schema.json`, this doc |

## Vercel deploy (dashboard)

The live site 404s if Vercel builds from the **repo root** — there is no Next.js app there.

**Required — Vercel Project Settings → General → Root Directory:**
```
apps/dashboard
```

Then **Redeploy** (Deployments → … → Redeploy).

**Environment variables** (Vercel → Settings → Environment Variables):

| Variable | Example | Purpose |
|----------|---------|---------|
| `CONTROL_API_URL` | `https://your-vps.example.com` | Server-side proxy for `/api/*` rewrites |
| `NEXT_PUBLIC_WS_URL` | `wss://your-vps.example.com/ws` | Control-plane WebSocket |
| `NEXT_PUBLIC_STREAM_URL` | `wss://your-stream.example.com/stream` | Terminal market stream |

Without `CONTROL_API_URL`, the UI shell loads but API calls fail. The Rust backends must run on a VPS — Vercel hosts the Next.js UI only.

```bash
# Deploy from CLI (after vercel login)
cd apps/dashboard && vercel --prod
```

## Local dev

```bash
# Terminal 1 — market stream (port 8080)
cargo run -p stream-api

# Terminal 1b — control plane (port 3001, optional)
cargo run -p control-api

# Terminal 2 — dashboard (port 3000)
cd apps/dashboard && npm run dev
```

Env:

- `CONTROL_API_PORT` — default `3001`
- `STREAM_API_PORT` — default `8080`
- `STREAM_BATCH_MS` — default `33` (clamped 25–50)
- `MOCK_SWAP_INTERVAL_MS` — mock swap cadence (default `80`)
- `NEXT_PUBLIC_WS_URL` — control UI: `ws://localhost:3001/ws`
- `NEXT_PUBLIC_STREAM_URL` — terminal: `ws://localhost:8080/stream`

## Extension points (future)

- Arbitrage execution → `crates/execution`, `apps/hotpath`
- Smart money → `crates/signals`
- Jito / Jupiter → cold path in `execution::hotpath`
