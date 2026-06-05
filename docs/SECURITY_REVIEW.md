# Security Review (automated scan summary)

This document summarizes an automated code scan performed across the workspace
and recommends prioritized remediations to reach a high security posture.

High-priority findings

1) Avoid panics in hot paths
- Files to review (examples):
  - `crates/engine/src/hotpath_runtime.rs`
  - `crates/engine/src/lib.rs`
  - `crates/execution/src/lib.rs`
  - `apps/worker/src/main.rs`
  - `crates/graph/src/lib.rs`
  - `crates/decoder/src/lib.rs`
  - `apps/control-api/src/main.rs`

  Replace `.expect()` / `.unwrap()` with proper error propagation and
  logging. Ensure services do not crash on transient RPC or I/O errors.

2) Locks and poison handling
- Files to review: `crates/orchestrator/src/coordinator.rs`, `crates/scalper/*`.
  Avoid `Mutex::lock().unwrap()` / `RwLock::write().unwrap()` in production
  hot paths. Handle poisoned locks explicitly and prefer channel/message
  passing where possible.

3) Secrets & key management
- `crates/wallet` zeroizes key material on drop — good. Ensure key files are
  never committed and are loaded only in live mode with clear startup checks.

4) Network exposure
- `apps/control-api` binds `0.0.0.0` by default. Enforce explicit config for
  production exposure and require TLS + authentication behind a reverse proxy.

Medium-priority findings

- Add `cargo-audit` and `cargo-deny` to CI (done) and fix reported issues.
- Add secret scanning (pre-commit, GitHub secret scanning) and `git-secrets`.
- Validate all external RPC responses and treat malformed RPC responses as
  recoverable errors rather than panics.

Next steps (recommended)

- Create a remediation branch and fix the top 10 non-test `.expect()` usages
  in hot paths to return `Result` and log errors.
- Add `cargo-audit` remediation: run locally `cargo audit fix` where possible.
- Add pre-commit hooks to detect secrets and large files.
- Run a dependency vulns triage and update vulnerable crates.
