/**
 * Orca Whirlpool price monitor.
 *
 * Watches pool state accounts via onAccountChange, decodes sqrtPriceX64 from
 * the on-chain Whirlpool account, computes the spot price, and emits an arb
 * opportunity event whenever the spread against a reference price exceeds
 * the configured threshold.
 *
 * Whirlpool account layout (Anchor, LE):
 *   [0..8]   discriminator
 *   [8..40]  whirlpoolsConfig (Pubkey)
 *   [40]     whirlpoolBump
 *   [41..43] tickSpacing (u16)
 *   [43..45] tickSpacingSeed ([u8;2])
 *   [45..47] feeRate (u16)
 *   [47..49] protocolFeeRate (u16)
 *   [49..65] liquidity (u128)
 *   [65..81] sqrtPrice (u128)   ← used here
 *   [81..85] tickCurrentIndex (i32)
 *   [85..93] protocolFeeOwedA (u64)
 *   [93..101] protocolFeeOwedB (u64)
 *   [101..133] tokenMintA (Pubkey)
 *   [181..213] tokenMintB (Pubkey)
 */
import { EventEmitter } from 'events';
/** Emitted when Orca spot price diverges from reference by more than threshold. */
export interface OrcaArbOpportunity {
    poolAddress: string;
    tokenMintA: string;
    tokenMintB: string;
    orcaSpotPrice: number;
    referencePrice: number;
    spreadBps: number;
    direction: 'buy_orca_sell_ref' | 'buy_ref_sell_orca';
    estimatedProfitBps: number;
    detectedAtMs: number;
}
export interface WhirlpoolPoolConfig {
    poolAddress: string;
    decimalsA: number;
    decimalsB: number;
    /** Threshold in bps before emitting an opportunity. Default 80 (0.8%). */
    spreadThresholdBps?: number;
}
export type PriceRef = (mintA: string, mintB: string) => number | undefined;
export declare class OrcaWhirlpoolMonitor extends EventEmitter {
    private readonly connection;
    private readonly subscriptionIds;
    private readonly pools;
    private readonly spreadThresholdBps;
    private getPriceRef;
    constructor(rpcUrl: string, getPriceRef: PriceRef, opts?: {
        spreadThresholdBps?: number;
    });
    watchPool(cfg: WhirlpoolPoolConfig): void;
    unwatchPool(poolAddress: string): void;
    stopAll(): void;
    updatePriceRef(fn: PriceRef): void;
    private handleAccountChange;
}
/**
 * Convert Orca sqrtPriceX64 (Q64.64) to the UI spot price of token A in terms of token B.
 *
 *   spotPrice_ui = (sqrtPriceX64 / 2^64)^2 × 10^(decimalsA − decimalsB)
 */
export declare function sqrtPriceX64ToUiPrice(sqrtPriceX64: bigint, decimalsA: number, decimalsB: number): number;
