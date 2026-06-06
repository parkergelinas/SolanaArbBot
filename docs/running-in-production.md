# Running in Production

## Prerequisites

- Node.js ≥ 18.17.0
- Rust toolchain (for `control-api` and `backtester`)
- A dedicated Solana RPC endpoint (Helius, QuikNode, or Triton)
- Wallet keypair with enough SOL to cover trades + fees

## 1 — Environment Setup

```bash
cp .env.example .env
# Fill in all required secrets (SOLANA_ARB_WALLET_KEY, HELIUS_API_KEY, SOLANA_RPC_URL)
```

Start with paper mode to validate connectivity:
```bash
BOT_PAPER_MODE=1 npm run dev --workspace=apps/bot
```

Watch the monitoring dashboard at `http://localhost:3333/status`.

When you are satisfied:
```bash
# Enable live trading
sed -i 's/BOT_PAPER_MODE=1/BOT_PAPER_MODE=0/' .env
```

## 2 — Install Dependencies

```bash
npm install --workspace=apps/bot
```

## 3 — Running with PM2 (recommended)

Install PM2 globally:
```bash
npm install -g pm2
```

Create an ecosystem file `ecosystem.config.cjs`:
```js
module.exports = {
  apps: [
    {
      name: 'solana-arb-bot',
      script: 'node',
      args: 'dist/index.js',
      cwd: './apps/bot',
      env_file: '.env',
      instances: 1,           // single-instance; multiple instances cause duplicate trades
      max_memory_restart: '512M',
      restart_delay: 5000,
      max_restarts: 10,
      exp_backoff_restart_delay: 100,
      out_file: './logs/bot-out.log',
      error_file: './logs/bot-err.log',
      merge_logs: true,
      log_date_format: 'YYYY-MM-DD HH:mm:ss Z',
    },
  ],
};
```

Build and start:
```bash
npm run build --workspace=apps/bot
pm2 start ecosystem.config.cjs
pm2 save          # persist across reboots
pm2 startup       # print the systemd/launchd command to run on startup
```

Useful PM2 commands:
```bash
pm2 logs solana-arb-bot       # tail logs
pm2 monit                     # live dashboard
pm2 restart solana-arb-bot    # rolling restart
pm2 stop solana-arb-bot       # graceful stop (waits for in-flight trades)
pm2 delete solana-arb-bot     # remove from PM2
```

## 4 — Running with systemd (alternative)

Create `/etc/systemd/system/solana-arb-bot.service`:

```ini
[Unit]
Description=Solana Arbitrage Bot
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/solana-arb-bot/apps/bot
EnvironmentFile=/opt/solana-arb-bot/.env
ExecStart=/usr/bin/node dist/index.js
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=solana-arb-bot
# Allow graceful shutdown — matches engine.gracefulStop(15000)
TimeoutStopSec=20s
# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=/opt/solana-arb-bot/apps/bot

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable --now solana-arb-bot
journalctl -u solana-arb-bot -f    # follow logs
```

## 5 — Monitoring

The bot exposes three HTTP endpoints on `MONITOR_PORT` (default 3333):

| Endpoint | Description |
|----------|-------------|
| `GET /health` | `200 OK` if running, `503` if halted by dead-man's switch |
| `GET /status` | JSON: uptime, wallet balance, total trades, PnL |
| `GET /trades?limit=50` | Last N trades from SQLite with full fee breakdown |

Add `/health` to your uptime monitor (e.g., Better Uptime, UptimeRobot, Grafana OnCall).

```bash
# Quick check
curl http://localhost:3333/health
curl http://localhost:3333/status | jq .
```

## 6 — Alerts

Set `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID` or `DISCORD_WEBHOOK_URL` in `.env`.
Alerts fire when:
- Session PnL drops below `ALERT_PNL_THRESHOLD_SOL` (default −0.1 SOL)
- Dead-man's switch halts trading after 3 consecutive transaction failures

To test your webhook:
```bash
node -e "
  import('./apps/bot/dist/monitoring/alerts.js').then(m =>
    m.sendAlert('🧪 test alert from production setup')
  );
"
```

## 7 — SQLite Trade Log

All trades are persisted to `SQLITE_PATH` (default `./trades.db`).

```bash
# Review recent trades
sqlite3 trades.db "SELECT timestamp, pair_label, profit_usd, success, failure_reason FROM trades ORDER BY timestamp DESC LIMIT 20;"

# Session PnL
sqlite3 trades.db "SELECT SUM(profit_usd) FROM trades;"

# Failure breakdown
sqlite3 trades.db "SELECT failure_reason, COUNT(*) FROM trades WHERE success=0 GROUP BY failure_reason;"
```

## 8 — Security Checklist

- [ ] `SOLANA_ARB_WALLET_KEY` loaded from a secret manager (not in `.env` on disk in production)
- [ ] `.env` file has mode `600` (`chmod 600 .env`)
- [ ] `CONTROL_API_KEY` is set (restricts the Rust control-api)
- [ ] `BOT_PAPER_MODE=0` only after full paper-mode validation
- [ ] RPC endpoint is private/authenticated (not the public fallback)
- [ ] `JITO_TIP_LAMPORTS` is less than your typical profit per trade
- [ ] `MIN_PROFIT_LAMPORTS` set high enough to cover worst-case fees

## 9 — Known Limitations and Remaining TODOs

| # | Item | Priority |
|---|------|----------|
| 1 | `@solana/web3.js` is on v1.x (EOL) — plan migration to v2 | High |
| 2 | `MeanReversionStrategy` is a scaffold — wire or remove | Medium |
| 3 | `JupiterTriggerClient` (TP/SL) is a stub — wire or remove | Low |
| 4 | `JupiterRecurringClient` (DCA) is a stub — wire or remove | Low |
| 5 | `extractMintFromSignature()` in launch-monitor returns `undefined` | Low |
| 6 | No runtime JSON schema validation on Jupiter/CMC responses | Medium |
| 7 | Helius WebSocket reconnection has no max-retry cap | Medium |
| 8 | `SOLANA_RPC_URL` not validated as parseable URL at startup | Low |
| 9 | Orca pool decimal config must be provided manually (no auto-fetch) | Low |
| 10 | No distributed rate limiting if running multiple bot instances | Medium |
