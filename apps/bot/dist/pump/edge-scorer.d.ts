import type { BondingCurveState } from './bonding-curve.js';
export type PumpEdgeSignalType = 'curve_jupiter_divergence' | 'graduation_proximity' | 'early_curve_entry' | 'migration_arb' | 'fresh_launch';
export interface PumpEdgeSignal {
    mint: string;
    signalType: PumpEdgeSignalType;
    /** Composite edge score 0–100. */
    edgeScore: number;
    /** Expected edge in basis points. */
    edgeBps: number;
    /** Curve spot price USD. */
    curvePriceUsd: number;
    /** External (Jupiter) price USD if available. */
    jupiterPriceUsd?: number;
    graduationPct: number;
    priceImpactBps: number;
    solRaised: number;
    complete: boolean;
    metadata: Record<string, unknown>;
}
export interface PumpEdgeConfig {
    /** Min edge bps to emit a signal. */
    minEdgeBps: number;
    /** Trade size in SOL for impact calculation. */
    tradeSol: number;
    /** Min graduation % for proximity plays. */
    minGraduationPct: number;
    /** Max graduation % for proximity plays. */
    maxGraduationPct: number;
    /** Max price impact bps for early entry. */
    maxEarlyImpactBps: number;
    /** Min divergence bps between curve and Jupiter. */
    minCurveJupiterDivergenceBps: number;
}
export declare const DEFAULT_PUMP_EDGE_CONFIG: PumpEdgeConfig;
/**
 * Score a pump.fun bonding curve for tradeable edge.
 *
 * Edge logic (from pump-public-docs + bonding curve math):
 * 1. Curve vs Jupiter divergence — buy cheap on curve, sell on Jupiter (or reverse)
 * 2. Graduation proximity — tokens 80–99% graduated often misprice vs post-migration AMM
 * 3. Early curve entry — fresh launches with low SOL raised + low price impact
 * 4. Migration arb — near-complete curves where curve price < Jupiter implies buy-before-migrate
 */
export declare function scorePumpEdge(state: BondingCurveState, solPriceUsd: number, jupiterPriceUsd: number | undefined, cfg?: PumpEdgeConfig): PumpEdgeSignal | null;
/** Rank multiple signals by edge score. */
export declare function rankPumpEdges(signals: PumpEdgeSignal[]): PumpEdgeSignal[];
