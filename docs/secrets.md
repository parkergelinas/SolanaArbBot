## Secure secrets handling

Do NOT paste or commit API keys, private keys, or other secrets into the repository.

Recommended local workflow:

- Add secrets to a local `.env` file (copy `.env.example` → `.env`) and never commit it.
- Use per-service env var names already supported by the repo, for example:
  - `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY` — Helius RPC/WebSocket key
  - `SOLSCAN_API_KEY` — Solscan researcher key
  - `NEXT_PUBLIC_HELIUS_API_KEY` — (optional) public key exposed to frontend (only if intended)

Deployment / CI:

- On Vercel: set environment variables in the Project Settings > Environment Variables UI.
- On GitHub Actions: add secrets under repository Settings > Secrets and reference them in workflows as `${{ secrets.MY_SECRET }}`.

Security tips:

- Never prefix secret env vars with `NEXT_PUBLIC_` unless the value is intentionally client-visible.
- Add `.env*` to `.gitignore` (already present).
- Rotate keys immediately if they were accidentally pasted into a chat or committed.
