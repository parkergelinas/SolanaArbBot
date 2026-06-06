---
name: project-arb-bot-v2-integration
description: Major feature additions to SolanaArbBot in June 2026 — Jupiter v6, Orca Whirlpool, hardened executor, Jito, dashboard, tests, production config
metadata:
  type: project
---

In June 2026 the following were added to apps/bot:

- **Jupiter v6 hardened client**: `execution/hardened-executor.ts` — signs transactions, dynamic CU estimation via simulation, exponential backoff retry, 5/8s timeouts.
- **Dead-man's switch**: `execution/dead-man-switch.ts` — halts after 3 consecutive failures, emits 'halt' event, operator reset() available.
- **Jito MEV protection**: `mev/jito-client.ts` — tip transfer injection into swap tx, random tip account, bundle submission, falls back to standard RPC, profit floor guard.
- **Orca Whirlpool monitor**: `orca/whirlpool-monitor.ts` — `onAccountChange` subscriptions, sqrtPriceX64 decoding at offset 65 (u128 LE), spread threshold alerting.
- **Opportunity detector**: `detector/opportunity-detector.ts` — multi-DEX price feeds, net profit after 2-leg fees + priority + tx fee, MIN_PROFIT_LAMPORTS filter, profitToRiskRatio ranking.
- **SQLite trade log**: `db/sqlite.ts` — WAL mode, schema with full trade metadata, used by monitoring server and hardened executor.
- **Express dashboard**: `monitoring/server.ts` — GET /health (503 if halted), /status (uptime/balance/PnL), /trades (last 50 from SQLite).
- **Alerts**: `monitoring/alerts.ts` — Telegram + Discord webhooks, ALERT_PNL_THRESHOLD_SOL check.
- **Pino logger**: `logger.ts` — LOG_LEVEL env, pino-pretty in dev.
- **SIGTERM/SIGINT**: `index.ts` — gracefulStop() drains in-flight trades before exit.
- **53 tests passing**: profit.test.ts, dead-man-switch.test.ts, opportunity-detector.test.ts (mocked Connection).
- **Production docs**: `docs/running-in-production.md` — PM2, systemd, security checklist, known TODOs.

**Why:** bot was in paper-mode only with no signing, no persistence, no observability.

**How to apply:** When working on this bot, assume the above files exist and are production-connected.
