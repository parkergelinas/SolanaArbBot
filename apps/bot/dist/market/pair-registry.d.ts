import type { BotEnv } from '../config/env.js';
import type { JupiterClient } from '../jupiter/client.js';
import type { ScanPair } from '../strategy/arb-scanner.js';
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
export declare class PairRegistry {
    private readonly client;
    private readonly cfg;
    private pairs;
    private meta;
    private rotationIndex;
    private loadedAtMs;
    constructor(client: JupiterClient, cfg: PairRegistryConfig);
    get allPairs(): readonly ScanPair[];
    get pairMeta(): readonly LoadedPairMeta[];
    /** Rotate through pairs — returns next batch for rate-limited scanning. */
    nextBatch(batchSize: number): ScanPair[];
    /** Load or refresh the pair universe. */
    refresh(force?: boolean): Promise<ScanPair[]>;
    private resolveTokens;
    private loadFromCmc;
    private loadFromStatic;
}
export declare function pairRegistryFromEnv(client: JupiterClient, env: BotEnv): PairRegistry;
