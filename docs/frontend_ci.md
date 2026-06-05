# Frontend continuous update loop

Iterative safe-update system for `apps/dashboard`.

## Cycle (mandatory after every frontend change)

1. **Analyze** — inspect stream flow, store churn, widget render cost
2. **Change** — targeted improvement only (no per-event React state)
3. **Test** — `npm test` in `apps/dashboard`
4. **Build** — `npm run build`
5. **Contract** — `cargo test -p stream-api` when types change
6. **Validate** — confirm latency/stress suites green

One command:

```powershell
.\scripts\test-frontend.ps1
```

## Non-regression rules (enforced by tests)

| Rule | Test suite |
|------|------------|
| rAF batching, no sync WS handlers | `tests/stream/client.test.ts` |
| 25–50ms batch windows, seq order | `tests/stream/batching.test.ts` |
| Bounded buffers (500 swaps, 80 signals) | `tests/stores/marketStore.test.ts` |
| 10k event stress, no memory blow-up | `tests/stress/rendering-stress.test.ts` |
| Render apply < 150ms on bursts | `tests/latency/pipeline.test.ts` |

## Failure protocol

If tests fail or UI degrades:

1. Stop iteration
2. Identify layer (client / store / component)
3. Revert violating pattern
4. Re-run `.\scripts\test-frontend.ps1`

## Architecture invariant

```
WebSocket → StreamClient (buffer) → rAF flush → Zustand applyMessages → memoized selectors → widgets
```

React state is **connection UI only** (`streamStore.connected`). Market data never touches `useState` per message.
