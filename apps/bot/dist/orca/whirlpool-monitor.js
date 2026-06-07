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
import { Connection, PublicKey } from '@solana/web3.js';
import { EventEmitter } from 'events';
import { logger } from '../logger.js';
const SQRT_PRICE_OFFSET = 65;
const TOKEN_MINT_A_OFFSET = 101;
const TOKEN_MINT_B_OFFSET = 181;
export class OrcaWhirlpoolMonitor extends EventEmitter {
    connection;
    subscriptionIds = new Map();
    pools = new Map();
    spreadThresholdBps;
    getPriceRef;
    constructor(rpcUrl, getPriceRef, opts = {}) {
        super();
        this.connection = new Connection(rpcUrl, 'processed');
        this.getPriceRef = getPriceRef;
        this.spreadThresholdBps = opts.spreadThresholdBps
            ?? Number(process.env.SPREAD_THRESHOLD_BPS ?? '80');
    }
    watchPool(cfg) {
        if (this.subscriptionIds.has(cfg.poolAddress))
            return;
        this.pools.set(cfg.poolAddress, cfg);
        const pubkey = new PublicKey(cfg.poolAddress);
        const id = this.connection.onAccountChange(pubkey, (accountInfo) => this.handleAccountChange(cfg.poolAddress, accountInfo, cfg), 'processed');
        this.subscriptionIds.set(cfg.poolAddress, id);
        logger.info({ pool: cfg.poolAddress }, 'orca: watching whirlpool');
    }
    unwatchPool(poolAddress) {
        const id = this.subscriptionIds.get(poolAddress);
        if (id === undefined)
            return;
        this.connection.removeAccountChangeListener(id).catch(() => undefined);
        this.subscriptionIds.delete(poolAddress);
        this.pools.delete(poolAddress);
    }
    stopAll() {
        for (const addr of [...this.subscriptionIds.keys()]) {
            this.unwatchPool(addr);
        }
    }
    updatePriceRef(fn) {
        this.getPriceRef = fn;
    }
    handleAccountChange(poolAddress, accountInfo, cfg) {
        try {
            const { sqrtPriceX64, tokenMintA, tokenMintB } = decodeWhirlpool(accountInfo.data);
            const orcaSpotPrice = sqrtPriceX64ToUiPrice(sqrtPriceX64, cfg.decimalsA, cfg.decimalsB);
            const refPrice = this.getPriceRef(tokenMintA, tokenMintB);
            if (refPrice === undefined || refPrice <= 0 || orcaSpotPrice <= 0)
                return;
            const spreadBps = Math.round(Math.abs(orcaSpotPrice - refPrice) / refPrice * 10_000);
            const threshold = cfg.spreadThresholdBps ?? this.spreadThresholdBps;
            if (spreadBps < threshold)
                return;
            const direction = orcaSpotPrice < refPrice ? 'buy_orca_sell_ref' : 'buy_ref_sell_orca';
            // Rough profit estimate: spread minus a 0.3% fee each leg
            const feeBps = 60; // 2 legs × 0.3%
            const estimatedProfitBps = Math.max(0, spreadBps - feeBps);
            const opportunity = {
                poolAddress,
                tokenMintA,
                tokenMintB,
                orcaSpotPrice,
                referencePrice: refPrice,
                spreadBps,
                direction,
                estimatedProfitBps,
                detectedAtMs: Date.now(),
            };
            logger.info(opportunity, 'orca: arb opportunity detected');
            this.emit('opportunity', opportunity);
        }
        catch (err) {
            logger.debug({ err, pool: poolAddress }, 'orca: failed to decode account');
        }
    }
}
// ─── decoding helpers ─────────────────────────────────────────────────────────
function decodeWhirlpool(data) {
    if (data.length < 214) {
        throw new Error(`whirlpool account too small: ${data.length} bytes`);
    }
    return {
        sqrtPriceX64: readU128LE(data, SQRT_PRICE_OFFSET),
        tokenMintA: new PublicKey(data.subarray(TOKEN_MINT_A_OFFSET, TOKEN_MINT_A_OFFSET + 32)).toBase58(),
        tokenMintB: new PublicKey(data.subarray(TOKEN_MINT_B_OFFSET, TOKEN_MINT_B_OFFSET + 32)).toBase58(),
    };
}
function readU128LE(buf, offset) {
    let result = 0n;
    for (let i = 0; i < 16; i++) {
        result |= BigInt(buf[offset + i]) << BigInt(8 * i);
    }
    return result;
}
/**
 * Convert Orca sqrtPriceX64 (Q64.64) to the UI spot price of token A in terms of token B.
 *
 *   spotPrice_ui = (sqrtPriceX64 / 2^64)^2 × 10^(decimalsA − decimalsB)
 */
export function sqrtPriceX64ToUiPrice(sqrtPriceX64, decimalsA, decimalsB) {
    const Q64 = 2n ** 64n;
    const sqrtFloat = Number(sqrtPriceX64) / Number(Q64);
    const rawPrice = sqrtFloat * sqrtFloat;
    return rawPrice * Math.pow(10, decimalsA - decimalsB);
}
