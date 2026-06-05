# Arbitrage Engine

Real-time cross-DEX arbitrage detection consuming price streams, computing spreads, and routing `ArbSignal`s to the execution engine.

## Architecture

```
Market Stream → Price Normalizer → Pool State Engine → Spread Calculator → Arb Signal Generator → (WS / channel → execution-engine)
```

| Module | Role |
|--------|------|
| `ingest.rs` | Mock or upstream price feed (50ms mock default) |
| `normalizer.rs` | Validate + canonicalize token pair order |
| `pool_state.rs` | DashMap `(pair, dex)` — price, liquidity, volatility EMA |
| `spread.rs` | Cross-DEX spread detection per pair |
| `signal.rs` | `ArbSignal` + `TradeSignal` mapping |
| `router.rs` | Profit/confidence/liquidity filters → output channel |

## Latency budget

| Stage | Target |
|-------|--------|
| normalize + pool update | <5ms |
| spread scan | <20ms |
| signal + route | <10ms |
| **total detection** | **<50ms** |

Batched recompute every `ARB_BATCH_INTERVAL_MS` (default 40ms). Incremental pool updates only — no full-state rebuild per tick.

## Thresholds

| Variable | Default | Description |
|----------|---------|-------------|
| `ARB_MIN_SPREAD_PCT` | 0.003 | Min spread (0.3%) to trigger |
| `ARB_MIN_LIQUIDITY_USD` | 10000 | Min pool liquidity |
| `ARB_MAX_STALE_MS` | 3000 | Reject stale quotes |
| `ARB_MIN_PROFIT_USD` | 1.0 | Min net profit after fees |
| `ARB_MIN_CONFIDENCE` | 0.7 | Min confidence score |
| `ARB_ROUND_TRIP_FEE_PCT` | 0.003 | Estimated round-trip fees (0.3%) |
| `ARB_BATCH_INTERVAL_MS` | 40 | Spread recompute interval |
| `ARB_MOCK_INTERVAL_MS` | 50 | Mock price tick interval |
| `ARB_WS_PORT` | 8091 | Optional WS broadcast port |
| `ARB_WS_ENABLED` | true | Enable `/arb` WebSocket |
| `ARB_MAX_EXECUTION_SIZE_USD` | 500 | Cap execution size |

## Run

```bash
# Terminal 1 — arb detection + WS
cargo run -p arb-engine

# Terminal 2 — execution engine (subscribes to arb WS when enabled)
ARB_WS_URL=ws://127.0.0.1:8091/arb cargo run -p execution-engine
```

## Integration

- **WS**: `ws://localhost:8091/arb` broadcasts JSON `ArbSignal`
- **Execution**: `execution-engine` maps `ArbSignal` → `TradeSignal` with `strategy: arbitrage_capture`
- **Contracts**: `shared/contracts/arb/v1.ts`

## Tests

```bash
cargo test -p arb-engine
```

Includes spread calc, stale reject, liquidity filter, and `detection_latency_test` (<50ms).
