import { SOL_MINT, USDC_MINT } from '../config/env.js';
import { CoinMarketCapClient } from './coinmarketcap.js';
import { STATIC_SOLANA_TOKENS, staticMintForSymbol } from './solana-mint-map.js';
/** Manages the scan pair universe — CMC top 100 mapped to Solana mints. */
export class PairRegistry {
    client;
    cfg;
    pairs = [];
    meta = [];
    rotationIndex = 0;
    loadedAtMs = 0;
    constructor(client, cfg) {
        this.client = client;
        this.cfg = cfg;
    }
    get allPairs() {
        return this.pairs;
    }
    get pairMeta() {
        return this.meta;
    }
    /** Rotate through pairs — returns next batch for rate-limited scanning. */
    nextBatch(batchSize) {
        if (this.pairs.length === 0)
            return [];
        const size = Math.min(batchSize, this.pairs.length);
        const batch = [];
        for (let i = 0; i < size; i++) {
            const idx = (this.rotationIndex + i) % this.pairs.length;
            batch.push(this.pairs[idx]);
        }
        this.rotationIndex = (this.rotationIndex + size) % this.pairs.length;
        return batch;
    }
    /** Load or refresh the pair universe. */
    async refresh(force = false) {
        const ttlMs = 3_600_000;
        if (!force && this.pairs.length > 0 && Date.now() - this.loadedAtMs < ttlMs) {
            return this.pairs;
        }
        const tokens = await this.resolveTokens();
        const quoteMint = this.cfg.quoteMint === 'USDC' ? USDC_MINT : SOL_MINT;
        const quoteDecimals = this.cfg.quoteMint === 'USDC' ? 6 : 9;
        const quoteSymbol = this.cfg.quoteMint;
        const pairs = [
            {
                label: 'SOL/USDC',
                baseMint: SOL_MINT,
                quoteMint: USDC_MINT,
                baseDecimals: 9,
                quoteDecimals: 6,
            },
        ];
        const meta = [
            { symbol: 'SOL', mint: SOL_MINT, source: 'core' },
        ];
        const seen = new Set([SOL_MINT, USDC_MINT]);
        for (const token of tokens) {
            if (seen.has(token.mint))
                continue;
            if (token.mint === quoteMint)
                continue;
            seen.add(token.mint);
            pairs.push({
                label: `${token.symbol}/${quoteSymbol}`,
                baseMint: token.mint,
                quoteMint,
                baseDecimals: token.decimals,
                quoteDecimals,
            });
            meta.push({
                symbol: token.symbol,
                mint: token.mint,
                cmcRank: token.cmcRank,
                source: token.source,
            });
            if (pairs.length >= this.cfg.maxPairs)
                break;
        }
        this.pairs = pairs;
        this.meta = meta;
        this.loadedAtMs = Date.now();
        return pairs;
    }
    async resolveTokens() {
        if (this.cfg.pairSource === 'cmc' && this.cfg.cmcApiKey) {
            try {
                return await this.loadFromCmc();
            }
            catch (err) {
                console.warn('[pair-registry] CMC load failed, falling back to static:', err);
            }
        }
        return this.loadFromStatic();
    }
    async loadFromCmc() {
        const cmc = new CoinMarketCapClient(this.cfg.cmcApiKey);
        const listings = await cmc.fetchTopListings(this.cfg.cmcTopN);
        const solanaTokens = await cmc.resolveSolanaMints(listings);
        const jupiterTokens = await this.client.fetchVerifiedTokens(5000).catch(() => []);
        const decimalsByMint = new Map(jupiterTokens.map((t) => [t.id, t.decimals]));
        const out = [];
        for (const t of solanaTokens) {
            const decimals = decimalsByMint.get(t.mint) ??
                staticMintForSymbol(t.symbol)?.decimals ??
                6;
            out.push({
                symbol: t.symbol,
                mint: t.mint,
                decimals,
                cmcRank: t.cmcRank,
                source: 'cmc',
            });
        }
        if (out.length < 10) {
            const staticFallback = this.loadFromStatic();
            const merged = [...out];
            const seen = new Set(out.map((t) => t.mint));
            for (const s of await staticFallback) {
                if (!seen.has(s.mint))
                    merged.push(s);
            }
            return merged;
        }
        return out;
    }
    async loadFromStatic() {
        const jupiterTokens = await this.client.fetchVerifiedTokens(5000).catch(() => []);
        const verifiedMints = new Set(jupiterTokens.map((t) => t.id));
        // When the token API is unavailable (429/offline), trust the pre-curated static list
        // entirely rather than silently shrinking the pair universe.
        const useVerified = verifiedMints.size > 0;
        return STATIC_SOLANA_TOKENS.filter((t) => t.symbol !== 'SOL' && t.symbol !== 'USDC' && t.symbol !== 'USDT')
            .filter((t) => !useVerified || verifiedMints.has(t.mint) || t.cmcRankHint !== undefined)
            .map((t) => ({
            symbol: t.symbol,
            mint: t.mint,
            decimals: t.decimals,
            cmcRank: t.cmcRankHint,
            source: 'static',
        }));
    }
}
export function pairRegistryFromEnv(client, env) {
    return new PairRegistry(client, {
        maxPairs: env.maxScanPairs,
        pairSource: env.pairSource,
        cmcApiKey: env.cmcApiKey,
        cmcTopN: env.cmcTopN,
        quoteMint: env.pairQuoteMint,
    });
}
