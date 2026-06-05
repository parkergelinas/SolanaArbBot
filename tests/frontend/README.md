# Frontend test suite

Continuous validation for `apps/dashboard` streaming + Zustand architecture.

## Run

```bash
cd apps/dashboard
npm test              # single run
npm run test:watch    # watch mode
npm run test:ci       # build + test (pre-deploy gate)
```

From repo root (PowerShell):

```powershell
.\scripts\test-frontend.ps1
```

## Coverage areas

| Suite | Path | Validates |
|-------|------|-----------|
| Stream client | `tests/stream/client.test.ts` | rAF batching, no sync subscriber calls |
| Batching protocol | `tests/stream/batching.test.ts` | 25–50ms window, seq ordering |
| Zustand store | `tests/stores/marketStore.test.ts` | caps, arb, isolation |
| Stress | `tests/stress/rendering-stress.test.ts` | 10k events, burst load |
| Latency | `tests/latency/pipeline.test.ts` | ingest/render budgets |

## Continuous update loop

After **any** frontend change:

1. `npm test` in `apps/dashboard`
2. `npm run build` in `apps/dashboard`
3. Optional: `cargo test -p stream-api` for contract alignment

Fail → stop, fix, re-run full suite.

## Non-regression rules enforced by tests

- No unbounded `swaps` / `signals` / candle history growth
- WS messages batched via rAF (not per-message handlers)
- Store `applyMessages` render path < 150ms for 500–1000 msg batches
