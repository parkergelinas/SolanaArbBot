/** CoinMarketCap Pro API — top listings + Solana contract resolution. */
const CMC_BASE = 'https://pro-api.coinmarketcap.com';
// SEC-3/SEC-4: validate mint address format before trusting it downstream.
// CMC contract addresses are user-supplied data from the CMC database and should
// not be passed to PublicKey() or used in PDAs without this check.
function isValidSolanaMint(address) {
    return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(address);
}
export class CoinMarketCapClient {
    apiKey;
    constructor(apiKey) {
        this.apiKey = apiKey;
    }
    headers() {
        return {
            Accept: 'application/json',
            'X-CMC_PRO_API_KEY': this.apiKey,
        };
    }
    /** Fetch top N cryptocurrencies by market cap. */
    async fetchTopListings(limit = 100) {
        const url = new URL(`${CMC_BASE}/v1/cryptocurrency/listings/latest`);
        url.searchParams.set('start', '1');
        url.searchParams.set('limit', String(limit));
        url.searchParams.set('sort', 'market_cap');
        url.searchParams.set('sort_dir', 'desc');
        url.searchParams.set('convert', 'USD');
        const resp = await fetch(url, { headers: this.headers() });
        if (!resp.ok) {
            const body = await resp.text().catch(() => '');
            throw new Error(`CMC listings HTTP ${resp.status}: ${body.slice(0, 200)}`);
        }
        const json = (await resp.json());
        return (json.data ?? []).map((row) => ({
            id: row.id,
            name: row.name,
            symbol: row.symbol,
            slug: row.slug,
            cmcRank: row.cmc_rank,
            priceUsd: row.quote?.USD?.price ?? 0,
            marketCapUsd: row.quote?.USD?.market_cap ?? 0,
            volume24hUsd: row.quote?.USD?.volume_24h ?? 0,
        }));
    }
    /**
     * Resolve Solana mint addresses for CMC coin IDs via `/v2/cryptocurrency/info`.
     * Picks the first Solana-platform contract per coin.
     */
    async resolveSolanaMints(listings) {
        if (listings.length === 0)
            return [];
        const ids = listings.map((l) => l.id).join(',');
        const url = new URL(`${CMC_BASE}/v2/cryptocurrency/info`);
        url.searchParams.set('id', ids);
        url.searchParams.set('aux', 'contract_address');
        const resp = await fetch(url, { headers: this.headers() });
        if (!resp.ok) {
            const body = await resp.text().catch(() => '');
            throw new Error(`CMC info HTTP ${resp.status}: ${body.slice(0, 200)}`);
        }
        const json = (await resp.json());
        const byId = new Map(listings.map((l) => [l.id, l]));
        const out = [];
        for (const entry of Object.values(json.data ?? {})) {
            const listing = byId.get(entry.id);
            if (!listing)
                continue;
            const solContract = (entry.contract_address ?? []).find((c) => c.platform?.name?.toLowerCase() === 'solana' ||
                c.platform?.coin?.symbol?.toUpperCase() === 'SOL');
            if (!solContract?.contract_address)
                continue;
            // Reject any address that doesn't look like a valid Solana pubkey.
            if (!isValidSolanaMint(solContract.contract_address))
                continue;
            out.push({
                cmcId: entry.id,
                symbol: entry.symbol,
                name: entry.name,
                mint: solContract.contract_address,
                cmcRank: listing.cmcRank,
                priceUsd: listing.priceUsd,
                marketCapUsd: listing.marketCapUsd,
            });
        }
        return out.sort((a, b) => a.cmcRank - b.cmcRank);
    }
}
