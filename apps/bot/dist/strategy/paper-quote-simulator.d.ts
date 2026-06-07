/**
 * Paper-mode route divergence simulator.
 *
 * When Jupiter's quote API is rate-limited (429), this generates synthetic
 * RouteDivergenceSnapshots using live price data from DexScreener.
 *
 * Model:
 *  - "restricted"  = single-hop route through a high-fee venue (Raydium v4: 0.25%)
 *    → round-trip COST (< 1:1 return), as expected
 *  - "unrestricted" = multi-hop route that exploits a simulated venue price
 *    inefficiency (30–80 bps above spot). This models what Jupiter's smart
 *    routing actually does in live markets.
 *
 * The stochastic "venue inefficiency" seeds from (scanTick × pairMint), giving
 * each pair a slowly-varying simulated opportunity rather than constant noise.
 *
 * PAPER MODE ONLY — never used in live trading.
 */
import type { ScanPair } from './arb-scanner.js';
import type { RouteDivergenceSnapshot } from '../market/state.js';
/**
 * Simulate route divergence for a single pair using current price data.
 *
 * @param pair          The trading pair (baseMint/quoteMint + decimals)
 * @param amountUi      Trade size in base-token UI units (e.g. 0.35 SOL)
 * @param priceUsd      USD price of baseMint
 * @param quotePriceUsd USD price of quoteMint (use 1.0 for USDC)
 * @param scanTick      Monotonic scan counter — seeds the stochastic inefficiency
 */
export declare function simulatePaperDivergence(pair: ScanPair, amountUi: number, priceUsd: number, quotePriceUsd: number, scanTick: number): RouteDivergenceSnapshot | null;
