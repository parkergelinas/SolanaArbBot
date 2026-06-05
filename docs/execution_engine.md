# Execution Engine

Low-latency Rust trade execution: signals → router → strategies → Jupiter → lifecycle.

## Architecture

```
Signal Engine (intelligence / demo feed)
        ↓
Execution Router (confidence, edge, size gates)
        ↓
Strategy Layer (ScalpStrategy | ArbStrategy)
        ↓
Jupiter Executor (quote + swap API)
        ↓
Order Lifecycle (Pending → Submitted → Confirmed | Failed)
        ↓
Audit Log (tracing + optional JSONL)
```

## Package

`backend/execution-engine` — binary `execution-engine`

## Paper mode (default)

**`PAPER_MODE=true` by default.** The engine fetches Jupiter quotes but never broadcasts transactions.

Set `PAPER_MODE=false` only when ready for live trading (requires keypair + `live` feature build).

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PAPER_MODE` | `true` | Simulate swaps, no broadcast |
| `MIN_SIGNAL_CONFIDENCE` | `0.55` | Router gate |
| `MIN_EXPECTED_EDGE_BPS` | `5.0` | Minimum edge bps |
| `MAX_TRADE_SIZE_USD` | `500.0` | Max per-trade notional |
| `JUPITER_QUOTE_URL` | `https://quote-api.jup.ag/v6/quote` | Quote endpoint |
| `JUPITER_SWAP_URL` | `https://quote-api.jup.ag/v6/swap` | Swap endpoint |
| `EXECUTION_AUDIT_PATH` | — | Optional JSONL audit file |
| `EXECUTION_KEYPAIR_PATH` | — | Keypair for live mode |
| `DEMO_SIGNAL_INTERVAL_MS` | `5000` | Demo signal cadence |

## Run

```bash
# Paper mode (default) — demo signals every 5s
cargo run -p execution-engine

# With audit log
set EXECUTION_AUDIT_PATH=./data/execution_audit.jsonl
cargo run -p execution-engine
```

## Integration

- Stub subscriber maps future intelligence-api whale alerts → `TradeSignal`
- Respects `system_contract.md` paper-only posture via `PAPER_MODE`
- Reuses `orchestrator` gate patterns for future risk integration

## Contracts

`shared/contracts/execution/v1.ts` — `TradeSignal`, `OrderRecord`, `OrderStatus`

## Tests

```bash
cargo test -p execution-engine
```

Unit tests cover router validation and order state machine. No live mainnet transactions.
