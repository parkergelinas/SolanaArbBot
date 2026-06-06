import { Connection } from '@solana/web3.js';
import { fetchBondingCurve, fetchBondingCurves } from '../pump/client.js';
import { DEFAULT_PUMP_EDGE_CONFIG, rankPumpEdges, scorePumpEdge, } from '../pump/edge-scorer.js';
import { heliusWsUrl, PumpLaunchMonitor, } from '../pump/launch-monitor.js';
import { freshBondingCurve } from '../pump/bonding-curve.js';
import { SOL_MINT } from '../config/env.js';
/**
 * Pump.fun edge strategy — exploits bonding curve mispricing vs Jupiter.
 *
 * Edge sources (from pump-public-docs):
 * - Uniswap V2 bonding curve with 1% fee → predictable slippage
 * - Graduation at ~85 SOL → migration arb window to PumpSwap
 * - Fresh launches on curve before Jupiter routes exist
 */
export class PumpEdgeStrategy {
    client;
    rpcUrl;
    cfg;
    id = 'pump_edge';
    connection;
    launchMonitor = null;
    recentLaunches = new Map();
    watchMints = new Set();
    constructor(client, rpcUrl, cfg = DEFAULT_PUMP_EDGE_CONFIG, heliusApiKey) {
        this.client = client;
        this.rpcUrl = rpcUrl;
        this.cfg = cfg;
        this.connection = new Connection(rpcUrl, 'confirmed');
        if (heliusApiKey) {
            this.launchMonitor = new PumpLaunchMonitor(heliusWsUrl(heliusApiKey), (ev) => {
                this.recentLaunches.set(ev.mint, ev);
                this.watchMints.add(ev.mint);
                if (this.recentLaunches.size > 200) {
                    const oldest = [...this.recentLaunches.keys()][0];
                    if (oldest)
                        this.recentLaunches.delete(oldest);
                }
            });
            this.launchMonitor.start();
        }
    }
    /** Add mints to watch for curve divergence (e.g. from pair registry). */
    watchMint(mint) {
        this.watchMints.add(mint);
    }
    stop() {
        this.launchMonitor?.stop();
    }
    async scan(state) {
        const mintsToScan = [
            ...this.watchMints,
            ...this.recentLaunches.keys(),
        ].slice(0, 20);
        if (mintsToScan.length === 0) {
            return { ...state, pumpEdges: [] };
        }
        const curves = await fetchBondingCurves(this.connection, mintsToScan);
        const priceMints = [...curves.keys()];
        const jupiterPrices = priceMints.length > 0
            ? await this.client.getPrices(priceMints).catch(() => ({}))
            : {};
        const signals = [];
        for (const [mint, curve] of curves) {
            if (curve.complete)
                continue;
            const jupPrice = jupiterPrices[mint]?.usdPrice;
            const signal = scorePumpEdge(curve, state.solPriceUsd, jupPrice, this.cfg);
            if (signal)
                signals.push(signal);
        }
        for (const [mint, launch] of this.recentLaunches) {
            if (curves.has(mint))
                continue;
            const ageMs = Date.now() - launch.detectedAtMs;
            if (ageMs > 120_000)
                continue;
            const curve = await fetchBondingCurve(this.connection, mint);
            const state_ = curve ?? freshBondingCurve(mint);
            const signal = scorePumpEdge(state_, state.solPriceUsd, undefined, this.cfg);
            if (signal) {
                signals.push({ ...signal, signalType: 'fresh_launch', metadata: { ...signal.metadata, launchAgeMs: ageMs } });
            }
        }
        return {
            ...state,
            pumpEdges: rankPumpEdges(signals),
        };
    }
    evaluate(state, ctx) {
        const edges = state.pumpEdges;
        if (!edges || edges.length === 0)
            return null;
        const best = edges[0];
        const expectedProfitUsd = (best.edgeBps / 10_000) * this.cfg.tradeSol * (state.solPriceUsd || 100);
        const rejectionReasons = [];
        if (best.edgeBps < this.cfg.minEdgeBps)
            rejectionReasons.push('below_min_edge');
        if (best.priceImpactBps > 500 && best.signalType !== 'graduation_proximity') {
            rejectionReasons.push(`high_impact:${best.priceImpactBps}`);
        }
        if (expectedProfitUsd < ctx.minProfitUsd)
            rejectionReasons.push('below_min_profit');
        const tradeSolLamports = BigInt(Math.round(this.cfg.tradeSol * 1e9));
        return {
            strategyId: this.id,
            pairLabel: `PUMP/${best.mint.slice(0, 8)}`,
            inputMint: SOL_MINT,
            outputMint: best.mint,
            amountInAtomic: tradeSolLamports.toString(),
            expectedOutAtomic: '0',
            netProfitUsd: expectedProfitUsd,
            grossSpreadBps: best.edgeBps,
            routeQualityScore: best.edgeScore,
            freshnessScore: best.signalType === 'fresh_launch' ? 95 : 70,
            rejectionReason: rejectionReasons.length > 0 ? rejectionReasons.join(',') : undefined,
            metadata: {
                signalType: best.signalType,
                graduationPct: best.graduationPct,
                curvePriceUsd: best.curvePriceUsd,
                jupiterPriceUsd: best.jupiterPriceUsd,
                priceImpactBps: best.priceImpactBps,
                solRaised: best.solRaised,
                ...best.metadata,
            },
        };
    }
}
