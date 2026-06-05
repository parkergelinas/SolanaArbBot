# Backend — Solana Intelligence Stack

```
backend/
├── data-layer/        # Ingest → parse → normalize → enrich → route
├── intelligence-api/  # Whale detection, wallet tracker, WS broker (:8090)
└── execution-engine/  # Signal router → Jupiter executor (paper default)
```

## Quick start

```bash
cargo run -p intelligence-api
# ws://localhost:8090/intelligence
```

See [docs/data_layer.md](../docs/data_layer.md) for architecture and env vars.
