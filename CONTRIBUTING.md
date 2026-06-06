Contributing
============

Thanks for contributing! A few guidelines to get started:

- Fork the repo and open pull requests against `main`.
- Run `cargo build --workspace` and `cargo test --workspace` before submitting changes.
- Frontend: run `pnpm install` and `pnpm test` in `apps/dashboard` or `frontend`.
- Follow existing coding patterns and add tests for non-trivial logic.
- For security-sensitive changes (wallet/key handling, live trading), include a brief security review note.

See `AGENTS.md` for agent-driven workflows and `ONBOARDING.md` for local setup.
