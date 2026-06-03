# AGENTS.md

Guidance for AI agents working in this repository.

## Cursor Cloud specific instructions

### Repository status

This repository is currently a **stub**: it contains only `README.md` (title: SolanaArbBot) and `.gitattributes`. There is no application source, dependency manifests (`package.json`, `Cargo.toml`, etc.), Docker/devcontainer config, or documented lint/test/run commands yet.

Until implementation lands on `main`, there are **no services to start** and no end-to-end product flow to exercise in the cloud VM.

### VM toolchain (preinstalled)

The Cursor Cloud VM already provides common tooling useful for a future Solana arbitrage bot:

| Tool | Notes |
|------|--------|
| Node.js | v22 via nvm (`node`, `npm`, `pnpm`, `yarn`) |
| Rust | `rustc` / `cargo` (1.83) |
| Python | 3.12 (`python3`) |

`docker` and `solana` CLI are **not** assumed to be installed unless added to the repo or documented in README.

### Lint / test / build / run

No project scripts exist yet. When manifests are added, prefer commands defined in:

- `package.json` scripts (Node/TypeScript)
- `Makefile` targets
- `cargo test` / `cargo run` (Rust)
- `README.md` setup section

Do not invent ports or service topology until compose/config files exist in the tree.

### Startup update script

On each cloud agent session, the VM runs a guarded dependency refresh (see SetupVmEnvironment `update_script`). It no-ops safely on the current stub and will install/fetch when lockfiles appear.

### When code is added

1. Re-read `README.md` and any new `AGENTS.md` / `CONTRIBUTING.md` sections.
2. Run the documented install command once if the update script is insufficient.
3. Start only the services required for the change (RPC, local validator, databases, etc.) as documented in the repo—do not guess from the project name alone.
