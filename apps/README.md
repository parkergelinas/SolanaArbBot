Apps
----

This directory contains runnable services and applications used for development and production.

Subfolders:
- `backtester/` — backtesting runner (Rust)
- `bot/` — TypeScript Jupiter quote-arb bot (`api.jup.ag` Swap v1)
- `control-api/` — HTTP control API for runtime operations (Rust)
- `dashboard/` — Next.js dashboard frontend (React/TypeScript)
- `hotpath/` — low-latency hotpath service (Rust)
- `stream-api/` — streaming API / WS broker (Rust)
- `worker/` — worker processes and background jobs (Rust)

Each app folder contains a crate or package. See the subfolder README (if present) or the crate's `Cargo.toml` / `package.json` for run instructions.
