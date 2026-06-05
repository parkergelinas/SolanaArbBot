# Expect/Unwrap Hotspots

Automated scan found multiple occurrences of `.expect()` / `.unwrap()` and
panic calls in non-test runtime code. Below are prioritized locations to fix.

Critical (hot paths / startup):

- `crates/engine/src/hotpath_runtime.rs` — startup and hot-loop `.expect()` calls.
- `crates/engine/src/lib.rs` — engine startup `.expect()`.
- `crates/execution/src/lib.rs` — simulation `.expect()` in runtime paths.
- `apps/worker/src/main.rs` — backpressure and task startup `.expect()`.
- `apps/control-api/src/main.rs` — binds 0.0.0.0; config load uses `?` but other parts assume env parsing.
- `crates/graph/src/lib.rs` — multiple `.expect()` when building and querying edges.
- `crates/decoder/src/*.rs` — decoding uses `.expect()` for parsing transforms.
- `crates/wallet/src/keypair.rs` — `sign_message` panics in paper mode; convert to Result or ensure upstream validation.
- `crates/orchestrator/src/coordinator.rs` — `RwLock` unwraps around state map.

Recommendation: For each location
- Convert `.expect()` to `?` returning `Result` and propagate errors to top-level with retries or graceful shutdown.
- Replace `Mutex::lock().unwrap()` with handling for `PoisonError`.
- Add unit/integration tests for failure cases to prevent regressions.
