import Database from 'better-sqlite3';
import path from 'path';

export interface TradeRecord {
  id?: number;
  timestamp: number;
  strategy_id: string;
  pair_label: string;
  input_mint: string;
  output_mint: string;
  amount_in_atomic: string;
  expected_out_atomic: string;
  actual_out_atomic: string | null;
  signature: string | null;
  profit_usd: number | null;
  success: number;            // 1 = success, 0 = failure
  failure_reason: string | null;
  priority_fee_lamports: number | null;
  jito_tip_lamports: number | null;
  compute_units_used: number | null;
  simulation_passed: number;  // 1 = yes, 0 = no
}

export interface TradeSummary {
  total: number;
  successful: number;
  failed: number;
}

const DB_PATH = process.env.SQLITE_PATH ?? path.join(process.cwd(), 'trades.db');

let _db: Database.Database | null = null;

export function getDb(): Database.Database {
  if (!_db) {
    _db = new Database(DB_PATH);
    _db.pragma('journal_mode = WAL');
    _db.pragma('synchronous = NORMAL');
    _db.pragma('foreign_keys = ON');
    initSchema(_db);
  }
  return _db;
}

function initSchema(db: Database.Database): void {
  db.exec(`
    CREATE TABLE IF NOT EXISTS trades (
      id                    INTEGER PRIMARY KEY AUTOINCREMENT,
      timestamp             INTEGER NOT NULL,
      strategy_id           TEXT    NOT NULL,
      pair_label            TEXT    NOT NULL,
      input_mint            TEXT    NOT NULL,
      output_mint           TEXT    NOT NULL,
      amount_in_atomic      TEXT    NOT NULL,
      expected_out_atomic   TEXT    NOT NULL,
      actual_out_atomic     TEXT,
      signature             TEXT,
      profit_usd            REAL,
      success               INTEGER NOT NULL DEFAULT 0,
      failure_reason        TEXT,
      priority_fee_lamports INTEGER,
      jito_tip_lamports     INTEGER,
      compute_units_used    INTEGER,
      simulation_passed     INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_trades_ts  ON trades (timestamp DESC);
    CREATE INDEX IF NOT EXISTS idx_trades_ok  ON trades (success);
  `);
}

export function insertTrade(trade: Omit<TradeRecord, 'id'>): number {
  const stmt = getDb().prepare(`
    INSERT INTO trades (
      timestamp, strategy_id, pair_label, input_mint, output_mint,
      amount_in_atomic, expected_out_atomic, actual_out_atomic,
      signature, profit_usd, success, failure_reason,
      priority_fee_lamports, jito_tip_lamports, compute_units_used, simulation_passed
    ) VALUES (
      @timestamp, @strategy_id, @pair_label, @input_mint, @output_mint,
      @amount_in_atomic, @expected_out_atomic, @actual_out_atomic,
      @signature, @profit_usd, @success, @failure_reason,
      @priority_fee_lamports, @jito_tip_lamports, @compute_units_used, @simulation_passed
    )
  `);
  const result = stmt.run(trade);
  return result.lastInsertRowid as number;
}

export function getRecentTrades(limit = 50): TradeRecord[] {
  return getDb()
    .prepare('SELECT * FROM trades ORDER BY timestamp DESC LIMIT ?')
    .all(limit) as TradeRecord[];
}

export function getTotalPnlUsd(): number {
  const row = getDb()
    .prepare('SELECT COALESCE(SUM(profit_usd), 0) AS total FROM trades')
    .get() as { total: number };
  return row.total;
}

export function getTradeSummary(): TradeSummary {
  const row = getDb()
    .prepare(`
      SELECT
        COUNT(*)                                          AS total,
        SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END)    AS successful,
        SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END)    AS failed
      FROM trades
    `)
    .get() as TradeSummary;
  return row;
}

export function closeDb(): void {
  _db?.close();
  _db = null;
}
