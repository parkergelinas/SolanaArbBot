/**
 * Capital tracker — persists simulated/live balance across restarts and
 * emits adaptive strategy parameters that scale automatically as capital grows.
 *
 * Design philosophy:
 *   The binding constraint at small capital is FIXED transaction cost (~$0.002/tx).
 *   At $0.002 fixed cost, you need:
 *     $0.01 profit target → spread ≥ 102 bps on $0.10 notional  (impractical)
 *     $0.01 profit target → spread ≥ 10  bps on $1.00 notional  (marginal)
 *     $0.01 profit target → spread ≥ 2   bps on $5.00 notional  (achievable)
 *   So the strategy gets MORE viable as capital grows — not less.
 *
 * Capital stages (SOL balance):
 *   Stage 1:  1–3  SOL — minimum viable, target $0.01–$0.05/trade
 *   Stage 2:  3–10 SOL — $0.05–$0.20/trade
 *   Stage 3: 10–30 SOL — $0.10–$0.50/trade
 *   Stage 4: 30+   SOL — $0.20–$1.00/trade, full strategy suite
 */
import Database from 'better-sqlite3';
const TABLE_DDL = `
  CREATE TABLE IF NOT EXISTS capital_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    capital_sol REAL NOT NULL,
    total_profit_usd REAL NOT NULL DEFAULT 0,
    trade_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
  );
`;
const INSERT_INITIAL = `
  INSERT OR IGNORE INTO capital_state (id, capital_sol, total_profit_usd, trade_count, updated_at)
  VALUES (1, ?, 0, 0, ?);
`;
const UPDATE = `
  UPDATE capital_state
  SET capital_sol = ?, total_profit_usd = ?, trade_count = ?, updated_at = ?
  WHERE id = 1;
`;
const SELECT = `SELECT capital_sol AS capitalSol, total_profit_usd AS totalProfitUsd, trade_count AS tradeCount FROM capital_state WHERE id = 1`;
export class CapitalTracker {
    db;
    state;
    solPriceUsd;
    constructor(dbPath, startingCapitalSol, solPriceUsd) {
        this.solPriceUsd = solPriceUsd;
        this.db = new Database(dbPath);
        this.db.exec(TABLE_DDL);
        this.db.prepare(INSERT_INITIAL).run(startingCapitalSol, Date.now());
        const row = this.db.prepare(SELECT).get();
        this.state = row ?? { capitalSol: startingCapitalSol, totalProfitUsd: 0, tradeCount: 0 };
    }
    get capitalSol() {
        return this.state.capitalSol;
    }
    /** Record a completed trade and update capital. profitUsd can be negative. */
    recordTrade(profitUsd, solPriceUsd) {
        const profitSol = profitUsd / solPriceUsd;
        this.state.capitalSol = Math.max(0.01, this.state.capitalSol + profitSol);
        this.state.totalProfitUsd += profitUsd;
        this.state.tradeCount += 1;
        this.db.prepare(UPDATE).run(this.state.capitalSol, this.state.totalProfitUsd, this.state.tradeCount, Date.now());
    }
    /**
     * Compute adaptive strategy parameters for the current capital level.
     *
     * The key insight: as capital doubles, the same spread generates 2× the
     * dollar profit per trade. This means we can tighten the spread threshold
     * over time and trade more frequently on tighter opportunities.
     */
    getAdaptiveParams(solPriceUsd) {
        const price = solPriceUsd ?? this.solPriceUsd;
        const capital = this.state.capitalSol;
        // Reserve 10% of capital for gas buffer — never trade the full stack
        const availableForTrade = capital * 0.9;
        if (capital < 3) {
            // Stage 1: 1–3 SOL
            // Fixed tx cost (~$0.003) needs to be < 30% of profit target ($0.01)
            // → need ≥ 10 bps on $0.30 notional — trade small but only on wide spreads
            return {
                tradeSizeUi: Math.min(availableForTrade, 0.5),
                minSpreadBps: 100, // very wide only — tight pairs not profitable at this size
                minProfitUsd: 0.01,
                stage: 'stage1:bootstrap',
                capitalSol: capital,
            };
        }
        if (capital < 10) {
            // Stage 2: 3–10 SOL
            // $0.05 target on 2 SOL ($130) = 3.8 bps — but pump pairs need 50+ bps
            return {
                tradeSizeUi: Math.min(availableForTrade, capital * 0.5),
                minSpreadBps: 60,
                minProfitUsd: 0.03,
                stage: 'stage2:growing',
                capitalSol: capital,
            };
        }
        if (capital < 30) {
            // Stage 3: 10–30 SOL
            // $0.10 target on 8 SOL ($520) = 1.9 bps — core pairs now viable
            return {
                tradeSizeUi: Math.min(availableForTrade, capital * 0.6),
                minSpreadBps: 35,
                minProfitUsd: 0.08,
                stage: 'stage3:established',
                capitalSol: capital,
            };
        }
        // Stage 4: 30+ SOL — full strategy, core + pump pairs
        return {
            tradeSizeUi: Math.min(availableForTrade, capital * 0.7),
            minSpreadBps: 25,
            minProfitUsd: 0.15,
            stage: 'stage4:full',
            capitalSol: capital,
        };
    }
    get stats() {
        return { ...this.state };
    }
    close() {
        this.db.close();
    }
}
