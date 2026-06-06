import type { TradeDecision } from '../strategy/types.js';
export interface StrategyRiskState {
    sessionLossUsd: number;
    consecutiveQuoteFailures: number;
    inventoryUsd: Record<string, number>;
    routeFamilyFailures: Record<string, number>;
    lastHaltReason?: string;
    cooldownUntilMs: number;
}
export interface StrategyRiskLimits {
    maxSessionLossUsd: number;
    maxInventoryUsdPerAsset: number;
    maxConsecutiveQuoteFailures: number;
    maxRouteFamilyFailures: number;
    cooldownAfterPoorExecutionMs: number;
    minSpreadBpsAfterCompression: number;
}
export declare const DEFAULT_RISK_LIMITS: StrategyRiskLimits;
export type RiskVerdict = 'allow' | 'reject' | 'halt';
export interface RiskCheckResult {
    verdict: RiskVerdict;
    reason?: string;
}
export declare function checkStrategyRisk(decision: TradeDecision, state: StrategyRiskState, limits?: StrategyRiskLimits, nowMs?: number): RiskCheckResult;
export declare function recordExecutionOutcome(state: StrategyRiskState, decision: TradeDecision, realizedUsd: number, success: boolean, limits?: StrategyRiskLimits, nowMs?: number): StrategyRiskState;
