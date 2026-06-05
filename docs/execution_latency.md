# Execution Decision Latency Budget

Decision latency is measured from **signal received** through **risk passed**, **Jupiter quote obtained**, and **order submitted** (or paper simulation complete).

## Budget breakdown

| Stage | Target | Notes |
|-------|--------|-------|
| strategy validate + size | <5ms | Sync only — no IO |
| risk gates | <2ms | Sync only — cooldown map lookup |
| Jupiter quote | <60ms | Async `reqwest`, 800ms timeout, retry on transient errors |
| swap build | <30ms | Async swap tx build (skipped in paper mode) |
| **total decision** | **<100ms** | Warn logged when exceeded |

## Instrumentation

`LatencyTracker` records per-stage timings and emits a `tracing::warn` when `total_ms > 100`:

```rust
struct LatencyBudget {
    strategy_ms: f64,
    risk_ms: f64,
    quote_ms: f64,
    swap_build_ms: f64,
    total_ms: f64,
}
```

## Performance rules (hot path)

- No `std::thread::sleep`, `blocking_recv`, or sync HTTP in the execution path
- Tokio async throughout: `tokio::sync::mpsc` for signals, `tokio::time::sleep` for backoff
- Shared `reqwest::Client` with connection pool; pre-warm on startup
- Jupiter retry: base 50ms exponential backoff, max 3 attempts, jitter — transient errors only (429, 5xx, timeout)
- Optional quote cache: `DashMap` keyed by `(input_mint, output_mint, amount)` with 200ms TTL (`QUOTE_CACHE_TTL_MS`)

## Environment

| Variable | Default | Effect |
|----------|---------|--------|
| `REQUEST_TIMEOUT_MS` | 800 | Jupiter quote/swap HTTP timeout |
| `QUOTE_CACHE_TTL_MS` | 200 | Repeat-signal quote cache TTL |
| `PAPER_MODE` | true | Skips swap broadcast; instant confirmation |

## Test

`cargo test -p execution-engine latency_budget_test` — mock Jupiter with 10ms delay; asserts full decision path stays under 100ms.
