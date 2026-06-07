/**
 * Telegram and Discord webhook alert delivery.
 * Fires on: PnL dropping below threshold, bot halt, custom messages.
 */
export declare function sendAlert(message: string): Promise<void>;
/**
 * Check PnL and send alert if it falls below the configured threshold.
 * Call this after every executed trade.
 */
export declare function checkPnlAlert(totalPnlUsd: number, solPriceUsd: number): Promise<void>;
export declare function alertHalt(reason: string): Promise<void>;
