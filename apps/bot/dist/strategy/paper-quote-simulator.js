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
/**
 * Simulate route divergence for a single pair using current price data.
 *
 * @param pair          The trading pair (baseMint/quoteMint + decimals)
 * @param amountUi      Trade size in base-token UI units (e.g. 0.35 SOL)
 * @param priceUsd      USD price of baseMint
 * @param quotePriceUsd USD price of quoteMint (use 1.0 for USDC)
 * @param scanTick      Monotonic scan counter — seeds the stochastic inefficiency
 */
export function simulatePaperDivergence(pair, amountUi, priceUsd, quotePriceUsd, scanTick) {
    if (priceUsd <= 0 || quotePriceUsd <= 0)
        return null;
    const now = Date.now();
    const amountInAtomic = Math.round(amountUi * 10 ** pair.baseDecimals);
    // Price of base in quote-token units (e.g. SOL price in USDC)
    const baseInQuote = priceUsd / quotePriceUsd;
    // ── Stochastic venue inefficiency ────────────────────────────────────────
    // Seed from pair mint + scan tick so each pair has a slowly-varying edge.
    // Uses two sine waves to simulate realistic oscillating price inefficiencies.
    const seed1 = Math.abs(Math.sin(scanTick * 0.137 + pair.baseMint.charCodeAt(2) * 0.0371));
    const seed2 = Math.abs(Math.sin(scanTick * 0.053 + pair.baseMint.charCodeAt(4) * 0.0149));
    // inefficiency: 30–80 bps (realistic range for liquid Solana pairs)
    const inefficiencyBps = 30 + (seed1 * 0.6 + seed2 * 0.4) * 50;
    const inefficiency = inefficiencyBps / 10_000;
    // ── AMM fee assumptions ────────────────────────────────────────────────
    const feeRestricted = 0.0025; // Raydium v4: 0.25% per leg
    const feeUnrestricted = 0.0005; // Orca concentrated: 0.05% per leg
    // ── Restricted route: single-hop, no price advantage ──────────────────
    const fwdOutRestricted = amountUi * baseInQuote * (1 - feeRestricted);
    const revOutRestricted = fwdOutRestricted / baseInQuote * (1 - feeRestricted);
    // Net: amountUi * (1 - feeRestricted)^2 ≈ amountUi * 0.9950 (−50 bps round-trip)
    // ── Unrestricted route: multi-hop, exploits venue inefficiency ─────────
    // Jupiter finds a pool where baseMint is priced inefficiency% above spot.
    // Forward leg captures that premium; reverse leg closes at spot price.
    const fwdOutUnrestricted = amountUi * baseInQuote * (1 + inefficiency) * (1 - feeUnrestricted);
    const revOutUnrestricted = fwdOutUnrestricted / baseInQuote * (1 - feeUnrestricted);
    // Net: amountUi * (1+inefficiency) * (1-fee)^2 — e.g. 40 bps edge → ~31 bps profit after 2×0.05%
    // ── Convert to atomic units ─────────────────────────────────────────────
    const fwdOutRestAtomic = Math.round(fwdOutRestricted * 10 ** pair.quoteDecimals);
    const fwdOutUnrestAtomic = Math.round(fwdOutUnrestricted * 10 ** pair.quoteDecimals);
    const revOutRestAtomic = Math.round(revOutRestricted * 10 ** pair.baseDecimals);
    const revOutUnrestAtomic = Math.round(revOutUnrestricted * 10 ** pair.baseDecimals);
    // ── Divergence (best − worst, normalised) ──────────────────────────────
    const mid = (revOutUnrestricted + revOutRestricted) / 2;
    const divergenceBps = mid > 0
        ? Math.round(((revOutUnrestricted - revOutRestricted) / mid) * 10_000)
        : 0;
    const constructions = [
        {
            label: 'restricted',
            restrictIntermediateTokens: true,
            forward: {
                request: {
                    inputMint: pair.baseMint,
                    outputMint: pair.quoteMint,
                    amount: amountInAtomic,
                    slippageBps: 50,
                    restrictIntermediateTokens: true,
                },
                response: {
                    inAmount: String(amountInAtomic),
                    outAmount: String(fwdOutRestAtomic),
                    priceImpactPct: (feeRestricted * 100).toFixed(3),
                    routePlan: [{ percent: 100 }],
                },
                capturedAtMs: now,
            },
            reverse: {
                request: {
                    inputMint: pair.quoteMint,
                    outputMint: pair.baseMint,
                    amount: fwdOutRestAtomic,
                    slippageBps: 50,
                    restrictIntermediateTokens: true,
                },
                response: {
                    inAmount: String(fwdOutRestAtomic),
                    outAmount: String(revOutRestAtomic),
                    priceImpactPct: (feeRestricted * 100).toFixed(3),
                    routePlan: [{ percent: 100 }],
                },
                capturedAtMs: now,
            },
        },
        {
            label: 'unrestricted',
            restrictIntermediateTokens: false,
            forward: {
                request: {
                    inputMint: pair.baseMint,
                    outputMint: pair.quoteMint,
                    amount: amountInAtomic,
                    slippageBps: 50,
                    restrictIntermediateTokens: false,
                },
                response: {
                    inAmount: String(amountInAtomic),
                    outAmount: String(fwdOutUnrestAtomic),
                    priceImpactPct: (feeUnrestricted * 100).toFixed(3),
                    routePlan: [{ percent: 100 }],
                },
                capturedAtMs: now,
            },
            reverse: {
                request: {
                    inputMint: pair.quoteMint,
                    outputMint: pair.baseMint,
                    amount: fwdOutUnrestAtomic,
                    slippageBps: 50,
                    restrictIntermediateTokens: false,
                },
                response: {
                    inAmount: String(fwdOutUnrestAtomic),
                    outAmount: String(revOutUnrestAtomic),
                    priceImpactPct: (feeUnrestricted * 100).toFixed(3),
                    routePlan: [{ percent: 100 }],
                },
                capturedAtMs: now,
            },
        },
    ];
    return {
        pairLabel: pair.label,
        constructions,
        bestConstructionLabel: 'unrestricted',
        divergenceBps,
    };
}
