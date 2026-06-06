# apps/web — Terminal Dashboard

This folder contains a small Next.js terminal-style dashboard for SolanaArbBot. It is designed to run on Vercel and uses Tailwind + a compact design system in `src/app/globals.css`.

## Local dev

Install dependencies and run the dev server:

```bash
cd apps/web
npm install
npm run dev
```

Open http://localhost:3000

## Build

```bash
npm run build
npm run start
```

## Environment (Vercel)

Recommended env vars (do NOT put private keys in NEXT_PUBLIC_* unless intended):

- `NEXT_PUBLIC_BOT_WS_URL` — WebSocket URL for bot status (e.g. wss://...)
- `NEXT_PUBLIC_RPC_URL` — (optional) public RPC for price lookups
- `NEXT_PUBLIC_HELIUS_API_KEY` — (optional) Helius API key for streaming (public usage)
- `NEXT_PUBLIC_ENABLE_MOCK_DATA` — `1` to use local mock data instead of WS

Vercel settings:

- Root: `apps/web`
- Build command: `npm run build`
- Output directory: `.next`

## Deployment notes

- Do not store private keys in repo or Vercel envs with `NEXT_PUBLIC_` prefix unless they are intended to be client-visible.
- For private server-side secrets, create server functions and keep secrets in Vercel's Environment Variables (do not prefix with NEXT_PUBLIC_). Example: `BOT_CONTROL_API_KEY` used by server functions.
- If you need persistent logging/metrics, wire OTLP or Prometheus exporters in backend services and use a hosted collector.

## Troubleshooting

- If styles look off, ensure `tailwind` is installed and `postcss` build runs. Run `npm run build` locally to reproduce Vercel behaviour.
