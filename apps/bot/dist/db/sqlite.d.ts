import Database from 'better-sqlite3';
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
    success: number;
    failure_reason: string | null;
    priority_fee_lamports: number | null;
    jito_tip_lamports: number | null;
    compute_units_used: number | null;
    simulation_passed: number;
}
export interface TradeSummary {
    total: number;
    successful: number;
    failed: number;
}
export declare function getDb(): Database.Database;
export declare function insertTrade(trade: Omit<TradeRecord, 'id'>): number;
export declare function getRecentTrades(limit?: number): TradeRecord[];
export declare function getTotalPnlUsd(): number;
export declare function getTradeSummary(): TradeSummary;
export declare function closeDb(): void;
