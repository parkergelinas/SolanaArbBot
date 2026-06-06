Onboarding
==========

Quick start for new contributors:

1. Install Rust (stable) and add components: `rustup component add clippy rustfmt`
2. Install Node 22+ and `corepack`/`pnpm`.
3. Clone the repo and run:

```bash
git clone git@github.com:parkergelinas/SolanaArbBot.git
cd SolanaArbBot
cargo build --workspace
cd apps/dashboard && pnpm install && pnpm dev
```

4. Run tests: `cargo test --workspace` and frontend tests in each app.

5. Read `docs/PROJECT_SUMMARY.md` and `AGENTS.md` for architecture and workflow.
