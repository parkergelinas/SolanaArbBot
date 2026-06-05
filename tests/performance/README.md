# Performance Tests

Latency and throughput benchmarks. Uses `criterion` or standalone harnesses.

## Targets

- Ingestion: ≥ 50k events/sec
- Pricing: < 1 ms per pool batch
- Routing: < 5 ms for 3-hop DFS across 1k-node graph

## Running

```bash
cargo bench
```
