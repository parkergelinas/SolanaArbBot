import Database from 'better-sqlite3';
import path from 'path';
const DB_PATH = process.env.SQLITE_PATH ?? path.join(process.cwd(), 'trades.db');
let _db = null;
export function getDb() {
    if (!_db) {
        _db = new Database(DB_PATH);
        _db.pragma('journal_mode = WAL');
        _db.pragma('synchronous = NORMAL');
        _db.pragma('foreign_keys = ON');
        initSchema(_db);
    }
    return _db;
}
function initSchema(db) {
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
export function insertTrade(trade) {
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
    return result.lastInsertRowid;
}
export function getRecentTrades(limit = 50) {
    return getDb()
        .prepare('SELECT * FROM trades ORDER BY timestamp DESC LIMIT ?')
        .all(limit);
}
export function getTotalPnlUsd() {
    const row = getDb()
        .prepare('SELECT COALESCE(SUM(profit_usd), 0) AS total FROM trades')
        .get();
    return row.total;
}
export function getTradeSummary() {
    const row = getDb()
        .prepare(`
      SELECT
        COUNT(*)                                          AS total,
        SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END)    AS successful,
        SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END)    AS failed
      FROM trades
    `)
        .get();
    return row;
}
export function closeDb() {
    _db?.close();
    _db = null;
}
