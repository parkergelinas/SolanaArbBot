# Deploying control-api + dashboard (production)

## Architecture (honest assessment)

| Component | Host | Why |
|-----------|------|-----|
| **dashboard** (`apps/dashboard`) | **Vercel** | Next.js static/SSR + `/api/*` rewrites |
| **control-api** (`apps/control-api`) | **Railway / Fly.io / Render / VPS** | Long-lived axum process, WebSocket `/ws`, autonomous bot loop, in-memory state |
| **stream-api** | Same VPS or separate host | Market stream `wss://…/stream` |
| **intelligence-api** | Same VPS or separate host | Whale/smart-money `wss://…/intelligence` |

**control-api cannot run on Vercel serverless.** Vercel Functions have short timeouts, no persistent WebSocket server for the bot loop, and no shared in-process state between invocations. The repo’s root `.vercelignore` already excludes Rust apps.

```
Browser ──REST──► Vercel dashboard ──rewrite──► https://control-api.example.com/api/*
Browser ──WS────► wss://control-api.example.com/ws          (NEXT_PUBLIC_WS_URL)
Browser ──WS────► wss://stream-api.example.com/stream       (NEXT_PUBLIC_STREAM_URL)
Browser ──WS────► wss://intelligence.example.com/intelligence (NEXT_PUBLIC_INTELLIGENCE_URL)
```

Secrets (RPC keys, keypairs) live on the **control-api host env**, never in `NEXT_PUBLIC_*` vars.

---

## 1. Deploy control-api (Railway / Fly / Render / VPS)

### Build

From repo root (Docker):

```bash
docker build -f apps/control-api/Dockerfile -t solana-control-api .
docker run --env-file apps/control-api/.env -p 3001:3001 solana-control-api
```

Or native:

```bash
cargo build --release -p control-api
CONTROL_API_HOST=0.0.0.0 CONTROL_API_PORT=3001 DEPLOY_ENV=production \
  SOLANA_ARB_RPC__ENDPOINTS=https://your-rpc.example.com \
  ./target/release/control-api
```

### Required host environment variables

| Variable | Required when | Description |
|----------|---------------|-------------|
| `DEPLOY_ENV` | production/staging | `production` enables strict startup validation |
| `CONTROL_API_HOST` | always | Bind address (`0.0.0.0` in containers) |
| `CONTROL_API_PORT` | always | Default `3001` |
| `SOLANA_ARB_RPC__ENDPOINTS` | production | Comma-separated JSON-RPC URLs |
| `SOLANA_ARB_WEBSOCKET__ENDPOINTS` | recommended | Comma-separated WSS URLs |
| `SOLANA_ARB_FEATURES__DRY_RUN` | always | `true` for paper (default safe) |
| `SOLANA_ARB_FEATURES__ENABLE_LIVE_TRADING` | live only | Must stay `false` until keypair configured |
| `SOLANA_ARB_WALLET__KEYPAIR_ENV_VAR` | live only | Name of env var holding base58 key |
| `RUST_LOG` | optional | e.g. `info,control_api=debug` |

See `apps/control-api/.env.example` for the full list (strategy flags, risk, signal engine, etc.).

### Optional upstream bridges

| Variable | Purpose |
|----------|---------|
| `INTELLIGENCE_WS_URL` | `wss://…/intelligence` — whale feed into signal-bus |
| `YELLOWSTONE_ENDPOINT` | Geyser gRPC for data-layer ingest |
| `WHALE_THRESHOLD_SOL` | Data-layer whale classification |
| `UPSTASH_REDIS_REST_URL` | Future shared signal buffer (stub logs today) |
| `SIGNAL_BUFFER_PATH` | Local JSONL persistence (single-instance only) |

### Startup validation

On boot, control-api exits with code **78** and prints numbered errors if:

- `DEPLOY_ENV=production` but `SOLANA_ARB_RPC__ENDPOINTS` is unset (public RPC only)
- Live trading enabled without keypair env/file
- `CONTROL_API_PORT` / `DEPLOY_ENV` / Redis URL malformed

Check health after deploy:

```bash
curl https://control-api.example.com/api/health
```

Response includes `bot_running`, `deploy_env`, `mode`, and `checks` (rpc, websocket, keypair).

### In-memory state limitation

Signals, trade journal, and bot runtime state are **in-memory** per process. Restarts lose unstored state. For multi-instance or restart survival:

- Set `UPSTASH_REDIS_REST_URL` (integration stub — wire `crates/signal-bus` persist next)
- Or accept single-replica deployment with `SIGNAL_BUFFER_PATH` JSONL on a mounted volume

---

## 2. Vercel dashboard setup

### Project settings

| Setting | Value |
|---------|-------|
| **Root Directory** | `apps/dashboard` |
| **Framework** | Next.js |

Redeploy after changing root directory or env vars.

### Environment variables

Set in **Vercel → Project → Settings → Environment Variables** for **Production** and **Preview** as needed:

| Variable | Example | Scope |
|----------|---------|-------|
| `CONTROL_API_URL` | `https://control-api.example.com` | Server only — powers `/api/*` rewrites |
| `NEXT_PUBLIC_WS_URL` | `wss://control-api.example.com/ws` | Browser WebSocket |
| `NEXT_PUBLIC_STREAM_URL` | `wss://stream-api.example.com/stream` | Terminal market stream |
| `NEXT_PUBLIC_INTELLIGENCE_URL` | `wss://intelligence.example.com/intelligence` | Whale panel |
| `NEXT_PUBLIC_SOLANA_NETWORK` | `mainnet` | Wallet adapter |
| `NEXT_PUBLIC_SOLANA_RPC` | `https://…` | Public RPC for wallet reads only |

**Do not** set `NEXT_PUBLIC_API_URL` in production unless you intentionally bypass the same-origin proxy (exposes API host to browsers).

### Important: redeploy after env changes

Vercel bakes `next.config.js` rewrites at **build time**. After adding or changing `CONTROL_API_URL`:

1. Save env vars in Vercel UI (or `vercel env pull` locally)
2. **Deployments → … → Redeploy** (or `cd apps/dashboard && vercel --prod`)

Without redeploy, API calls may still fail with 503 (`CONTROL_API_URL is not configured`).

### Pull env locally (optional)

```bash
cd apps/dashboard
vercel link          # once
vercel env pull .env.local
npm run dev
```

### Deploy CLI

```bash
cd apps/dashboard
vercel --prod
```

---

## 3. End-to-end verification

1. `curl https://<vercel-app>/api/health` → proxied control-api JSON
2. Open dashboard → WebSocket connects to `NEXT_PUBLIC_WS_URL`
3. `POST /api/system/start` starts bot (paper mode by default)
4. Vercel **Functions** logs show rewrite targets; control-api logs show bind + `deploy_env`

---

## 4. Security checklist

- [ ] `SOLANA_ARB_FEATURES__DRY_RUN=true` until paper trading validated
- [ ] Keypair only on control-api host secret store (never Vercel, never `NEXT_PUBLIC_*`)
- [ ] Dedicated paid RPC in production
- [ ] CORS: control-api uses permissive CORS today — restrict `tower-http` origins if exposing publicly
- [ ] TLS termination at Railway/Fly/Render load balancer (`wss://` for WebSocket)
