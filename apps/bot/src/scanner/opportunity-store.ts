import Database from 'better-sqlite3';
import path from 'path';

export interface ScannedOpportunity {
  id?: number;
  timestamp: number;
  strategy_id: string;
  pair_label: string;
  spread_bps: number;
  net_profit_usd: number;
  score: number;
  sim_pass: number;
  survival_200ms: number;
  capital_usd: number;
  raw_json: string;
}

export interface OpportunityStats {
  total: number;
  simPassRate: number;
  avgSpreadBps: number;
  avgNetProfitUsd: number;
  avgScore: number;
  byStrategy: Array<{ strategy_id: string; count: number; avg_net_profit_usd: number }>;
}

const DB_PATH = process.env.SCANNER_DB_PATH ?? path.join(process.cwd(), 'scanner.db');

let _db: Database.Database | null = null;

function getDb(): Database.Database {
  if (!_db) {
    _db = new Database(DB_PATH);
    _db.pragma('journal_mode = WAL');
    _db.pragma('synchronous = NORMAL');
    initSchema(_db);
  }
  return _db;
}

function initSchema(db: Database.Database): void {
  db.exec(`
    CREATE TABLE IF NOT EXISTS scanned_opportunities (
      id             INTEGER PRIMARY KEY AUTOINCREMENT,
      timestamp      INTEGER NOT NULL,
      strategy_id    TEXT    NOT NULL,
      pair_label     TEXT    NOT NULL,
      spread_bps     REAL    NOT NULL,
      net_profit_usd REAL    NOT NULL,
      score          REAL    NOT NULL,
      sim_pass       INTEGER NOT NULL DEFAULT 0,
      survival_200ms INTEGER NOT NULL DEFAULT 0,
      capital_usd    REAL    NOT NULL,
      raw_json       TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_so_ts       ON scanned_opportunities (timestamp DESC);
    CREATE INDEX IF NOT EXISTS idx_so_strategy ON scanned_opportunities (strategy_id);
  `);
}

export function insertOpportunity(opp: Omit<ScannedOpportunity, 'id'>): number {
  const stmt = getDb().prepare(`
    INSERT INTO scanned_opportunities
      (timestamp, strategy_id, pair_label, spread_bps, net_profit_usd, score,
       sim_pass, survival_200ms, capital_usd, raw_json)
    VALUES
      (@timestamp, @strategy_id, @pair_label, @spread_bps, @net_profit_usd, @score,
       @sim_pass, @survival_200ms, @capital_usd, @raw_json)
  `);
  return stmt.run(opp).lastInsertRowid as number;
}

export function getRecentOpportunities(limit = 100): ScannedOpportunity[] {
  return getDb()
    .prepare('SELECT * FROM scanned_opportunities ORDER BY timestamp DESC LIMIT ?')
    .all(limit) as ScannedOpportunity[];
}

export function getOpportunitiesSince(sinceMs: number): ScannedOpportunity[] {
  return getDb()
    .prepare('SELECT * FROM scanned_opportunities WHERE timestamp > ? ORDER BY timestamp DESC')
    .all(sinceMs) as ScannedOpportunity[];
}

export function getStats(): OpportunityStats {
  const db = getDb();
  const row = db
    .prepare(
      `SELECT
         COUNT(*)                                                      AS total,
         COALESCE(AVG(CASE WHEN sim_pass = 1 THEN 1.0 ELSE 0.0 END), 0) AS simPassRate,
         COALESCE(AVG(spread_bps), 0)                                  AS avgSpreadBps,
         COALESCE(AVG(net_profit_usd), 0)                              AS avgNetProfitUsd,
         COALESCE(AVG(score), 0)                                       AS avgScore
       FROM scanned_opportunities`,
    )
    .get() as {
    total: number;
    simPassRate: number;
    avgSpreadBps: number;
    avgNetProfitUsd: number;
    avgScore: number;
  };

  const byStrategy = db
    .prepare(
      `SELECT strategy_id, COUNT(*) AS count, AVG(net_profit_usd) AS avg_net_profit_usd
       FROM scanned_opportunities
       GROUP BY strategy_id
       ORDER BY count DESC
       LIMIT 10`,
    )
    .all() as Array<{ strategy_id: string; count: number; avg_net_profit_usd: number }>;

  return { ...row, byStrategy };
}

export function closeOpportunityDb(): void {
  _db?.close();
  _db = null;
}
