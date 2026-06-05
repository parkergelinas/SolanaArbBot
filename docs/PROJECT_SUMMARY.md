# Project Summary

SolanaArbBot is a Rust-based modular system for detecting and executing arbitrage
opportunities on Solana-based markets. The repository uses a workspace layout
with multiple crates responsible for ingestion, pricing, execution, risk,
and orchestration.

Status
- Implementation: in active development (multiple crates and a web dashboard).
- CI/CD: limited; some automation present in `.github/workflows`.

Key components
- `apps/dashboard`: Next.js dashboard for monitoring signals, backtests, and trades.
- `worker` and `crates/*`: Rust crates implementing ingestion, pricing, execution,
  risk engines, routing, and storage.
- `infrastructure` / `prometheus` / `grafana`: monitoring and alerting configurations.

Repository layout (high level)
- `crates/` — workspace crates grouped by domain (pricing, routing, risk, etc.).
- `apps/` — frontend and control API components.
- `docs/` — design documents, architecture notes, changelog.
- `infrastructure/` — monitoring and deployment helper configs.

Development
- Build all Rust crates: `cargo build --workspace`
- Run tests: `cargo test --workspace`
- Dashboard: follow `apps/dashboard/README` (install Node.js and run `pnpm install && pnpm dev` or `npm` equivalents).

Committing large files
- Avoid committing build artifacts and large binaries. Use `.gitignore` and
  Git LFS for necessary large assets. See repository root for initial guidance.

Useful references
- Changelog: `docs/CHANGELOG.md`
- System contract and architecture notes: `docs/system_contract.md`

Contact & next steps
- For contribution guidelines or to propose changes, open an issue or PR.
