#!/usr/bin/env npx tsx
/**
 * CLI analysis tool for the scanner SQLite dataset.
 *
 * Usage:
 *   npx tsx src/scanner/dataset.ts summary
 *   npx tsx src/scanner/dataset.ts spread-dist
 *   npx tsx src/scanner/dataset.ts break-even
 *   npx tsx src/scanner/dataset.ts hourly
 *   npx tsx src/scanner/dataset.ts export [--format=csv|ndjson|json] [--since=<ms>]
 */

import Database from 'better-sqlite3';
import path from 'path';

const DB_PATH = process.env.SCANNER_DB_PATH ?? path.join(process.cwd(), 'scanner.db');

function openDb(): Database.Database {
  try {
    return new Database(DB_PATH, { readonly: true });
  } catch {
    console.error(`Cannot open ${DB_PATH} — run the scanner first to populate data.`);
    process.exit(1);
  }
}

function parseArgs(): { command: string; format: string; since: number | null } {
  const args = process.argv.slice(2);
  const command = args[0] ?? 'summary';
  const formatArg = args.find((a) => a.startsWith('--format='));
  const sinceArg = args.find((a) => a.startsWith('--since='));
  return {
    command,
    format: formatArg ? formatArg.split('=')[1]! : 'json',
    since: sinceArg ? Number(sinceArg.split('=')[1]) : null,
  };
}

function pct(n: number): string {
  return (n * 100).toFixed(1) + '%';
}
function usd(n: number): string {
  return '$' + n.toFixed(4);
}

// ── Commands ─────────────────────────────────────────────────────────────────

function cmdSummary(db: Database.Database): void {
  const row = db
    .prepare(
      `SELECT
         COUNT(*) AS total,
         AVG(CASE WHEN sim_pass=1 THEN 1.0 ELSE 0.0 END) AS pass_rate,
         AVG(spread_bps)     AS avg_spread,
         AVG(net_profit_usd) AS avg_pnl,
         AVG(score)          AS avg_score,
         MIN(timestamp)      AS first_ts,
         MAX(timestamp)      AS last_ts
       FROM scanned_opportunities`,
    )
    .get() as {
    total: number;
    pass_rate: number;
    avg_spread: number;
    avg_pnl: number;
    avg_score: number;
    first_ts: number;
    last_ts: number;
  };

  const p50Spread = (
    db
      .prepare(
        `SELECT spread_bps FROM scanned_opportunities ORDER BY spread_bps LIMIT 1
         OFFSET (SELECT COUNT(*)/2 FROM scanned_opportunities)`,
      )
      .get() as { spread_bps: number } | undefined
  )?.spread_bps ?? 0;

  const p95Spread = (
    db
      .prepare(
        `SELECT spread_bps FROM scanned_opportunities ORDER BY spread_bps LIMIT 1
         OFFSET (SELECT CAST(COUNT(*)*0.95 AS INT) FROM scanned_opportunities)`,
      )
      .get() as { spread_bps: number } | undefined
  )?.spread_bps ?? 0;

  const byStrategy = db
    .prepare(
      `SELECT strategy_id, COUNT(*) AS cnt, AVG(net_profit_usd) AS avg_pnl
       FROM scanned_opportunities GROUP BY strategy_id ORDER BY cnt DESC LIMIT 5`,
    )
    .all() as Array<{ strategy_id: string; cnt: number; avg_pnl: number }>;

  const span =
    row.first_ts && row.last_ts
      ? ((row.last_ts - row.first_ts) / 3_600_000).toFixed(1) + 'h'
      : '—';

  console.log('═══ Scanner Dataset Summary ═══');
  console.log(`Total opportunities : ${row.total}`);
  console.log(`Time span           : ${span}`);
  console.log(`Sim-pass rate       : ${pct(row.pass_rate ?? 0)}`);
  console.log(`Avg spread bps      : ${(row.avg_spread ?? 0).toFixed(2)}`);
  console.log(`  p50 spread bps    : ${p50Spread.toFixed(2)}`);
  console.log(`  p95 spread bps    : ${p95Spread.toFixed(2)}`);
  console.log(`Avg net P&L         : ${usd(row.avg_pnl ?? 0)}`);
  console.log(`Avg score           : ${(row.avg_score ?? 0).toFixed(3)}`);
  console.log('');
  console.log('Top strategies:');
  for (const s of byStrategy) {
    console.log(`  ${s.strategy_id.padEnd(30)} ${String(s.cnt).padStart(6)} rows  avg P&L ${usd(s.avg_pnl)}`);
  }
}

function cmdSpreadDist(db: Database.Database): void {
  const buckets = [
    { label: '  0 – 5  bps', min: 0, max: 5 },
    { label: '  5 – 10 bps', min: 5, max: 10 },
    { label: ' 10 – 20 bps', min: 10, max: 20 },
    { label: ' 20 – 50 bps', min: 20, max: 50 },
    { label: ' 50+      bps', min: 50, max: 1e9 },
  ];

  const total = (db.prepare('SELECT COUNT(*) AS n FROM scanned_opportunities').get() as { n: number }).n;
  console.log('═══ Spread Distribution ═══');
  for (const b of buckets) {
    const cnt = (
      db
        .prepare('SELECT COUNT(*) AS n FROM scanned_opportunities WHERE spread_bps >= ? AND spread_bps < ?')
        .get(b.min, b.max) as { n: number }
    ).n;
    const bar = '█'.repeat(Math.round((cnt / (total || 1)) * 40));
    console.log(`${b.label} : ${String(cnt).padStart(6)} ${bar}`);
  }
}

function cmdBreakEven(db: Database.Database): void {
  const thresholds = [1, 2, 5, 10, 15, 20, 30, 50];
  console.log('═══ Break-Even Analysis ═══');
  console.log('Threshold bps  Total  Profitable  Rate');
  for (const t of thresholds) {
    const r = db
      .prepare(
        `SELECT
           COUNT(*) AS total,
           SUM(CASE WHEN net_profit_usd > 0 THEN 1 ELSE 0 END) AS profitable
         FROM scanned_opportunities WHERE spread_bps >= ?`,
      )
      .get(t) as { total: number; profitable: number };
    const rate = r.total > 0 ? pct(r.profitable / r.total) : '—';
    console.log(
      `  >= ${String(t).padStart(3)} bps  : ${String(r.total).padStart(6)}  ${String(r.profitable).padStart(10)}  ${rate}`,
    );
  }
}

function cmdHourly(db: Database.Database): void {
  const rows = db
    .prepare(
      `SELECT
         strftime('%Y-%m-%d %H:00', datetime(timestamp/1000, 'unixepoch')) AS hour,
         COUNT(*) AS cnt,
         AVG(net_profit_usd) AS avg_pnl,
         SUM(CASE WHEN sim_pass=1 THEN 1 ELSE 0 END) AS passes
       FROM scanned_opportunities
       WHERE timestamp > ?
       GROUP BY hour
       ORDER BY hour DESC`,
    )
    .all(Date.now() - 48 * 3_600_000) as Array<{
    hour: string;
    cnt: number;
    avg_pnl: number;
    passes: number;
  }>;

  console.log('═══ Hourly Breakdown (last 48h) ═══');
  console.log('Hour                Count  Passes  Avg P&L');
  for (const r of rows) {
    console.log(
      `  ${r.hour}  ${String(r.cnt).padStart(5)}  ${String(r.passes).padStart(6)}  ${usd(r.avg_pnl)}`,
    );
  }
  if (rows.length === 0) console.log('  (no data in last 48h)');
}

function cmdExport(
  db: Database.Database,
  format: string,
  since: number | null,
): void {
  const whereClause = since != null ? `WHERE timestamp > ${Date.now() - since}` : '';
  const rows = db
    .prepare(`SELECT * FROM scanned_opportunities ${whereClause} ORDER BY timestamp DESC`)
    .all() as Array<Record<string, unknown>>;

  if (format === 'csv') {
    const cols = [
      'id', 'timestamp', 'strategy_id', 'pair_label', 'spread_bps',
      'net_profit_usd', 'score', 'sim_pass', 'survival_200ms', 'capital_usd',
    ];
    process.stdout.write(cols.join(',') + '\n');
    for (const r of rows) {
      process.stdout.write(cols.map((c) => r[c]).join(',') + '\n');
    }
  } else if (format === 'ndjson') {
    for (const r of rows) {
      process.stdout.write(JSON.stringify(r) + '\n');
    }
  } else {
    process.stdout.write(JSON.stringify(rows, null, 2) + '\n');
  }
}

// ── Entry point ───────────────────────────────────────────────────────────────

const { command, format, since } = parseArgs();
const db = openDb();

switch (command) {
  case 'summary':
    cmdSummary(db);
    break;
  case 'spread-dist':
    cmdSpreadDist(db);
    break;
  case 'break-even':
    cmdBreakEven(db);
    break;
  case 'hourly':
    cmdHourly(db);
    break;
  case 'export':
    cmdExport(db, format, since);
    break;
  default:
    console.error(`Unknown command: ${command}`);
    console.error('Available: summary, spread-dist, break-even, hourly, export');
    process.exit(1);
}

db.close();
