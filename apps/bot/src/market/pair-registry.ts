import type { BotEnv } from '../config/env.js';
import { SOL_MINT, USDC_MINT } from '../config/env.js';
import type { JupiterClient } from '../jupiter/client.js';
import type { ScanPair } from '../strategy/arb-scanner.js';
import { CoinMarketCapClient } from './coinmarketcap.js';
import { STATIC_SOLANA_TOKENS, staticMintForSymbol } from './solana-mint-map.js';

export interface PairRegistryConfig {
  maxPairs: number;
  pairSource: 'cmc' | 'static';
  cmcApiKey?: string;
  cmcTopN: number;
  quoteMint: 'USDC' | 'SOL';
}

export interface LoadedPairMeta {
  symbol: string;
  mint: string;
  cmcRank?: number;
  source: 'cmc' | 'static' | 'core';
}

/** Manages the scan pair universe — CMC top 100 mapped to Solana mints. */
export class PairRegistry {
  private pairs: ScanPair[] = [];
  private meta: LoadedPairMeta[] = [];
  private rotationIndex = 0;
  private loadedAtMs = 0;

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: PairRegistryConfig,
  ) {}

  get allPairs(): readonly ScanPair[] {
    return this.pairs;
  }

  get pairMeta(): readonly LoadedPairMeta[] {
    return this.meta;
  }

  /** Rotate through pairs — returns next batch for rate-limited scanning. */
  nextBatch(batchSize: number): ScanPair[] {
    if (this.pairs.length === 0) return [];
    const size = Math.min(batchSize, this.pairs.length);
    const batch: ScanPair[] = [];
    for (let i = 0; i < size; i++) {
      const idx = (this.rotationIndex + i) % this.pairs.length;
      batch.push(this.pairs[idx]!);
    }
    this.rotationIndex = (this.rotationIndex + size) % this.pairs.length;
    return batch;
  }

  /** Load or refresh the pair universe. */
  async refresh(force = false): Promise<ScanPair[]> {
    const ttlMs = 3_600_000;
    if (!force && this.pairs.length > 0 && Date.now() - this.loadedAtMs < ttlMs) {
      return this.pairs;
    }

    const tokens = await this.resolveTokens();
    const quoteMint = this.cfg.quoteMint === 'USDC' ? USDC_MINT : SOL_MINT;
    const quoteDecimals = this.cfg.quoteMint === 'USDC' ? 6 : 9;
    const quoteSymbol = this.cfg.quoteMint;

    const pairs: ScanPair[] = [
      {
        label: 'SOL/USDC',
        baseMint: SOL_MINT,
        quoteMint: USDC_MINT,
        baseDecimals: 9,
        quoteDecimals: 6,
      },
    ];
    const meta: LoadedPairMeta[] = [
      { symbol: 'SOL', mint: SOL_MINT, source: 'core' },
    ];

    const seen = new Set([SOL_MINT, USDC_MINT]);

    for (const token of tokens) {
      if (seen.has(token.mint)) continue;
      if (token.mint === quoteMint) continue;
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

      if (pairs.length >= this.cfg.maxPairs) break;
    }

    this.pairs = pairs;
    this.meta = meta;
    this.loadedAtMs = Date.now();
    return pairs;
  }

  private async resolveTokens(): Promise<
    Array<{ symbol: string; mint: string; decimals: number; cmcRank?: number; source: 'cmc' | 'static' }>
  > {
    if (this.cfg.pairSource === 'cmc' && this.cfg.cmcApiKey) {
      try {
        return await this.loadFromCmc();
      } catch (err) {
        console.warn('[pair-registry] CMC load failed, falling back to static:', err);
      }
    }
    return this.loadFromStatic();
  }

  private async loadFromCmc(): Promise<
    Array<{ symbol: string; mint: string; decimals: number; cmcRank?: number; source: 'cmc' | 'static' }>
  > {
    const cmc = new CoinMarketCapClient(this.cfg.cmcApiKey!);
    const listings = await cmc.fetchTopListings(this.cfg.cmcTopN);
    const solanaTokens = await cmc.resolveSolanaMints(listings);

    const jupiterTokens = await this.client.fetchVerifiedTokens(5000).catch(() => []);
    const decimalsByMint = new Map(jupiterTokens.map((t) => [t.id, t.decimals]));

    const out: Array<{
      symbol: string;
      mint: string;
      decimals: number;
      cmcRank?: number;
      source: 'cmc' | 'static';
    }> = [];

    for (const t of solanaTokens) {
      const decimals =
        decimalsByMint.get(t.mint) ??
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
        if (!seen.has(s.mint)) merged.push(s);
      }
      return merged;
    }

    return out;
  }

  private async loadFromStatic(): Promise<
    Array<{ symbol: string; mint: string; decimals: number; cmcRank?: number; source: 'cmc' | 'static' }>
  > {
    const jupiterTokens = await this.client.fetchVerifiedTokens(5000).catch(() => []);
    const verifiedMints = new Set(jupiterTokens.map((t) => t.id));

    return STATIC_SOLANA_TOKENS.filter(
      (t) => t.symbol !== 'SOL' && t.symbol !== 'USDC' && t.symbol !== 'USDT',
    )
      .filter((t) => verifiedMints.has(t.mint) || t.cmcRankHint !== undefined)
      .map((t) => ({
        symbol: t.symbol,
        mint: t.mint,
        decimals: t.decimals,
        cmcRank: t.cmcRankHint,
        source: 'static' as const,
      }));
  }
}

export function pairRegistryFromEnv(client: JupiterClient, env: BotEnv): PairRegistry {
  return new PairRegistry(client, {
    maxPairs: env.maxScanPairs,
    pairSource: env.pairSource,
    cmcApiKey: env.cmcApiKey,
    cmcTopN: env.cmcTopN,
    quoteMint: env.pairQuoteMint,
  });
}
