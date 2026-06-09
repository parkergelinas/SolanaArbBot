import express, { type Express, type Request, type Response } from 'express';
import type { Server } from 'http';
import type { ScannedOpportunity } from './opportunity-store.js';

const DASHBOARD_HTML = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Scanner Live Feed</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0d1117; color: #c9d1d9; font-family: 'Courier New', Courier, monospace; font-size: 13px; }
  header { display: flex; align-items: center; gap: 24px; padding: 14px 20px; border-bottom: 1px solid #21262d; background: #161b22; }
  header h1 { font-size: 16px; color: #f0f6fc; letter-spacing: 0.05em; }
  .badge { background: #21262d; border: 1px solid #30363d; border-radius: 4px; padding: 2px 8px; font-size: 12px; color: #8b949e; }
  .badge span { color: #58a6ff; font-weight: bold; }
  #status { width: 8px; height: 8px; border-radius: 50%; background: #f85149; display: inline-block; margin-right: 6px; }
  #status.connected { background: #3fb950; }
  .table-wrap { overflow-x: auto; padding: 0 20px 20px; }
  table { width: 100%; border-collapse: collapse; margin-top: 16px; }
  thead th { text-align: left; padding: 6px 10px; border-bottom: 1px solid #21262d; color: #8b949e; font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; }
  tbody tr { border-bottom: 1px solid #161b22; transition: background 0.15s; }
  tbody tr:hover { background: #161b22; }
  tbody tr.pass td:first-child { border-left: 2px solid #3fb950; }
  tbody tr.fail td:first-child { border-left: 2px solid #30363d; }
  td { padding: 6px 10px; white-space: nowrap; }
  td.strategy { color: #79c0ff; }
  td.pair { color: #f0f6fc; }
  td.spread { color: #e3b341; }
  td.pnl.pos { color: #3fb950; }
  td.pnl.neg { color: #f85149; }
  td.score { color: #a5d6ff; }
  td.simpass.yes { color: #3fb950; }
  td.simpass.no { color: #8b949e; }
  td.surv.yes { color: #3fb950; }
  td.surv.no { color: #f85149; }
  td.time { color: #6e7681; }
  .empty { text-align: center; padding: 40px; color: #30363d; }
</style>
</head>
<body>
<header>
  <h1><span id="status"></span>Scanner Live Feed</h1>
  <div class="badge">clients <span id="clients">—</span></div>
  <div class="badge">scanned <span id="total">0</span></div>
  <div class="badge">sim-pass <span id="passcount">0</span></div>
</header>
<div class="table-wrap">
  <table>
    <thead>
      <tr>
        <th>Strategy</th><th>Pair</th><th>Spread bps</th>
        <th>Net P&amp;L</th><th>Score</th><th>Sim Pass</th>
        <th>Surv 200ms</th><th>Time</th>
      </tr>
    </thead>
    <tbody id="feed"><tr><td colspan="8" class="empty">Waiting for first scan…</td></tr></tbody>
  </table>
</div>
<script>
  let total = 0, passes = 0;
  const MAX_ROWS = 100;
  const tbody = document.getElementById('feed');
  const statusEl = document.getElementById('status');
  const clientsEl = document.getElementById('clients');
  const totalEl = document.getElementById('total');
  const passEl = document.getElementById('passcount');

  function fmt(n) { return n == null ? '—' : Number(n).toFixed(4); }
  function fmtBps(n) { return n == null ? '—' : Number(n).toFixed(1); }
  function fmtTime(ts) {
    const d = new Date(ts);
    return d.toTimeString().slice(0, 8);
  }

  function addRow(opp) {
    const pass = opp.sim_pass === 1 || opp.sim_pass === true;
    const surv = opp.survival_200ms === 1 || opp.survival_200ms === true;
    const pnl = parseFloat(opp.net_profit_usd);
    total++;
    if (pass) passes++;
    totalEl.textContent = total;
    passEl.textContent = passes;

    const tr = document.createElement('tr');
    tr.className = pass ? 'pass' : 'fail';
    tr.innerHTML =
      '<td class="strategy">' + opp.strategy_id + '</td>' +
      '<td class="pair">' + opp.pair_label + '</td>' +
      '<td class="spread">' + fmtBps(opp.spread_bps) + '</td>' +
      '<td class="pnl ' + (pnl >= 0 ? 'pos' : 'neg') + '">$' + fmt(pnl) + '</td>' +
      '<td class="score">' + fmtBps(opp.score) + '</td>' +
      '<td class="simpass ' + (pass ? 'yes' : 'no') + '">' + (pass ? '✓' : '✗') + '</td>' +
      '<td class="surv ' + (surv ? 'yes' : 'no') + '">' + (surv ? '✓' : '✗') + '</td>' +
      '<td class="time">' + fmtTime(opp.timestamp) + '</td>';

    if (tbody.firstChild && tbody.firstChild.colSpan) tbody.innerHTML = '';
    tbody.insertBefore(tr, tbody.firstChild);
    while (tbody.children.length > MAX_ROWS) tbody.removeChild(tbody.lastChild);
  }

  function connect() {
    const es = new EventSource('/events');
    es.addEventListener('open', () => { statusEl.className = 'connected'; });
    es.addEventListener('snapshot', (e) => {
      const rows = JSON.parse(e.data);
      rows.slice().reverse().forEach(addRow);
    });
    es.addEventListener('opportunity', (e) => { addRow(JSON.parse(e.data)); });
    es.addEventListener('stats', (e) => {
      const s = JSON.parse(e.data);
      if (s.clientCount != null) clientsEl.textContent = s.clientCount;
    });
    es.addEventListener('error', () => {
      statusEl.className = '';
      es.close();
      setTimeout(connect, 3000);
    });
  }
  connect();
</script>
</body>
</html>`;

type SseClient = Response;

export class SseServer {
  readonly app: Express;
  private server: Server | null = null;
  private readonly clients = new Set<SseClient>();
  private readonly keepAliveIntervals = new Map<SseClient, ReturnType<typeof setInterval>>();

  constructor() {
    this.app = express();
    this.app.disable('x-powered-by');

    this.app.get('/', (_req: Request, res: Response) => {
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.send(DASHBOARD_HTML);
    });
  }

  get clientCount(): number {
    return this.clients.size;
  }

  broadcast(event: string, data: unknown): void {
    const payload = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
    for (const client of this.clients) {
      client.write(payload);
    }
  }

  pushSnapshot(opportunities: ScannedOpportunity[]): void {
    // Called per-client on connect — send to the most recently connected client only.
    // Because we add before calling this, the last entry in the set is the new client.
    const clients = [...this.clients];
    const newest = clients[clients.length - 1];
    if (!newest) return;
    const payload = `event: snapshot\ndata: ${JSON.stringify(opportunities.slice(0, 20))}\n\n`;
    newest.write(payload);
  }

  start(port: number, onSnapshot: () => ScannedOpportunity[]): void {
    // Re-wire /events to send snapshot on connect using caller-provided getter
    this.app.get('/events', (req: Request, res: Response) => {
      res.setHeader('Content-Type', 'text/event-stream');
      res.setHeader('Cache-Control', 'no-cache');
      res.setHeader('Connection', 'keep-alive');
      res.setHeader('Access-Control-Allow-Origin', '*');
      res.setHeader('Access-Control-Allow-Headers', 'Cache-Control');
      res.flushHeaders();

      this.clients.add(res);

      const snapshot = onSnapshot();
      if (snapshot.length > 0) {
        res.write(`event: snapshot\ndata: ${JSON.stringify(snapshot)}\n\n`);
      }

      const ping = setInterval(() => {
        res.write(': ping\n\n');
      }, 25_000);
      this.keepAliveIntervals.set(res, ping);

      req.on('close', () => {
        clearInterval(ping);
        this.keepAliveIntervals.delete(res);
        this.clients.delete(res);
      });
    });

    this.server = this.app.listen(port);
  }

  async stop(): Promise<void> {
    for (const [, interval] of this.keepAliveIntervals) clearInterval(interval);
    this.keepAliveIntervals.clear();
    for (const client of this.clients) client.end();
    this.clients.clear();

    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => resolve());
      } else {
        resolve();
      }
    });
  }
}
