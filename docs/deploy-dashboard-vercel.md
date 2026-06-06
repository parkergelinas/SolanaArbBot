Deploying `apps/dashboard` to Vercel
====================================

This repository is a monorepo. The Next.js dashboard lives in `apps/dashboard`.

Quick steps
- Connect the repository to Vercel (Vercel dashboard).
- Ensure the project uses the root `vercel.json` in the repository — it is configured
  to build the `apps/dashboard` package.
- Add the following environment variables in the Vercel project settings (use
  secrets or project env variables):
  - `CONTROL_API_URL` (e.g. https://control-api.example.com) — optional but
    recommended for production API rewrites.
  - `NEXT_PUBLIC_STREAM_HTTP_URL` (e.g. https://stream-api.example.com) — used
    by the dashboard for scanner proxying.
  - `NEXT_PUBLIC_WS_URL` — websocket endpoint for real-time streams (if used).

Notes
- `vercel.json` already points the build to `apps/dashboard/package.json` and
  sets `installCommand`/`buildCommand` to run in that folder.
- `apps/dashboard` has Node engine requirement `>=18.17.0` in `package.json`.

Local build check
1. From repository root, install and build the dashboard locally:

```powershell
npm install --legacy-peer-deps --prefix apps/dashboard
npm run build --prefix apps/dashboard
```

2. Run locally:

```powershell
npm run dev --prefix apps/dashboard
```

If you want, I can run the local build check now.
