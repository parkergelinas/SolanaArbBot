import { JupiterClient } from '../jupiter/client.js';
import type { JupiterTokenMeta } from '../jupiter/types.js';
import type { BotEnv } from '../config/env.js';
import { type MarketFilterConfig, type MarketState } from './state.js';
export interface UniverseOptions {
    filter?: MarketFilterConfig;
    /** Extra mints always included (e.g. SOL, USDC). */
    alwaysInclude?: string[];
    tokenLimit?: number;
}
/** Build curated token universe from Jupiter Tokens API v2. */
export declare class TokenUniverse {
    private readonly client;
    private cache;
    private loadedAtMs;
    private readonly ttlMs;
    constructor(client: JupiterClient, ttlMs?: number);
    refresh(force?: boolean): Promise<JupiterTokenMeta[]>;
    curatedMints(opts?: UniverseOptions): Promise<string[]>;
}
/** Aggregates Jupiter Price v3 + Tokens v2 into a single `MarketState`. */
export declare class MarketDataLayer {
    private readonly client;
    private readonly universe;
    constructor(client: JupiterClient, universe: TokenUniverse);
    buildState(mints: string[], opts?: UniverseOptions): Promise<MarketState>;
}
export declare function createMarketStack(env: BotEnv): {
    client: JupiterClient;
    universe: TokenUniverse;
    dataLayer: MarketDataLayer;
};
