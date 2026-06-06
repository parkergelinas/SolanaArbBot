Crates
------

Reusable Rust libraries used across the workspace. Each subfolder is a cargo crate with domain-specific logic.

Notable crates:
- `accounts/` — Solana account models and helpers
- `pricing/` — price aggregation and feed adapters
- `routing/` — route/path selection and optimization
- `rpc_client/` — thin RPC wrapper and helpers
- `wallet/` — key management and signing abstractions

To build or run a crate directly, use `cargo build -p <crate-name>` from the repository root or from inside the crate folder.
