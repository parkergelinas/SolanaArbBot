# Latency Budget Model

Target: **50–150 ms perceived latency** from Solana event to pixels on screen.

## Stage budgets

| Stage | Budget | Owner | Notes |
|-------|--------|-------|-------|
| Solana → ingestion | 20–80 ms | `stream-api` ingestion | RPC/Geyser path; mock ~80 ms cadence |
| Rust processing | 5–20 ms | `market` engine | DashMap updates, no locks on read path |
| WS broadcast batch | 5–15 ms | `broker` | 25–50 ms window; amortized per event |
| Network (LAN) | 1–5 ms | infra | Production: colocate API + RPC |
| Frontend WS receive | 1–3 ms | `StreamClient` | Parse JSON batch |
| Frontend ingest buffer | 5–10 ms | `StreamClient` | rAF coalesce |
| React render | 16–33 ms | dashboard | One Zustand flush per frame |
| **Total perceived** | **50–150 ms** | system | Measured in `LatencyMonitor` |

## Measurement points

### Server (`ts_ms`)

Set at batch flush in `WSBatchFrame::new()` from `SystemTime::now()`.

### Client (`streamStore.latency`)

| Metric | Calculation | Budget |
|--------|-------------|--------|
| `wsMs` | `Date.now() - frame.ts_ms` | ≤ 15 ms (LAN) |
| `ingestMs` | `flushedAt - receivedAt` (rAF wait) | ≤ 10 ms |
| `renderMs` | `applyMessages` duration | ≤ 33 ms |
| `e2eMs` | frame receive → store commit | ≤ 150 ms |

Displayed in the terminal **Latency** bar with color thresholds.

## Optimization levers

1. **Reduce `STREAM_BATCH_MS`** toward 25 ms (server) — trades CPU for freshness.
2. **Colocate** stream-api with Solana RPC / Geyser plugin.
3. **Binary encoding** (v2) — cuts parse time and frame size.
4. **Client virtualisation** — swap feed uses `@tanstack/react-virtual`.
5. **No per-message React state** — mandatory; violations add 16+ ms per event.

## SLA tiers (future)

| Tier | Target p99 | Use case |
|------|------------|----------|
| Observation | 150 ms | Dashboard, research |
| Alerting | 80 ms | Signals, whale flow |
| Execution | 30 ms | Hot path (`apps/hotpath`) — separate process |

The dashboard terminal targets the **Observation** tier; execution uses `apps/hotpath` without JSON in the hot loop.
