# Alpha Pipeline Latency Budget

Cycle = input event → fusion → score → gate → queue push.

| Stage | Target |
|-------|--------|
| fusion | <5ms |
| scoring | <3ms |
| gate | <2ms |
| queue | <1ms |
| **total** | **<50ms** |

Instrumentation: `AlphaLatencyBudget` in `alpha-engine/src/latency.rs` — warns when `total_ms > 50`.

## Performance rules

- Incremental DashMap updates per token — no full recompute per event
- Drain tick only pops/expired from queue — does not rescore all tokens
- No `std::thread::sleep` in fusion path (mock inputs use sleep in separate thread only)
- Lock-free DashMap reads on hot path

## Test

`cargo test -p alpha-engine alpha_pipeline_latency_test`
