import {
  fetchLatestTokenProfiles,
  fetchTokenPairs,
  isPumpFunPair,
  pairAgeMinutes,
  searchDexPairs,
  type DexPair,
} from './dexscreener';
import type { PumpMomentumHit, PumpScannerResponse, ScannerMeta } from './types';

/** Pump.fun graduates bonding curve near ~$69k market cap. */
const GRADUATION_MCAP_USD = 69_000;
const PUMP_PROGRAM_ID = '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P';

interface OnChainLaunch {
  mint: string;
  signature: string;
  detectedAtMs: number;
}

/** Recent Pump.fun Create events via Helius enhanced transactions API. */
async function fetchOnChainLaunches(heliusKey: string): Promise<OnChainLaunch[]> {
  const url = `https://api.helius.xyz/v0/addresses/${PUMP_PROGRAM_ID}/transactions?api-key=${heliusKey}&limit=25`;
  const res = await fetch(url, { next: { revalidate: 0 } });
  if (!res.ok) return [];

  const txs = (await res.json()) as {
    signature?: string;
    timestamp?: number;
    type?: string;
    source?: string;
    tokenTransfers?: { mint?: string }[];
    accountData?: { account?: string; nativeBalanceChange?: number }[];
  }[];

  const out: OnChainLaunch[] = [];
  const seen = new Set<string>();

  for (const tx of txs) {
    const sig = tx.signature ?? '';
    if (!sig) continue;
    const isCreate =
      (tx.type ?? '').toLowerCase().includes('create') ||
      (tx.source ?? '').toLowerCase().includes('pump');
    if (!isCreate) continue;

    const mint =
      tx.tokenTransfers?.find((t) => t.mint && t.mint !== PUMP_PROGRAM_ID)?.mint ?? '';
    if (!mint || seen.has(mint)) continue;
    seen.add(mint);

    out.push({
      mint,
      signature: sig,
      detectedAtMs: (tx.timestamp ?? Math.floor(Date.now() / 1000)) * 1000,
    });
  }
  return out;
}

function onChainHitFromLaunch(
  launch: OnChainLaunch,
  pair?: DexPair,
): PumpMomentumHit | null {
  const pairHit = pair ? pairToPumpHit(pair) : null;
  const mint = launch.mint;
  const symbol = pair?.baseToken?.symbol ?? mint.slice(0, 6);

  if (pairHit) {
    return {
      ...pairHit,
      onChain: true,
      signature: launch.signature,
      momentumScore: Math.min(100, pairHit.momentumScore + 15),
      detectedAtMs: launch.detectedAtMs,
    };
  }

  const ageMin = Math.max(0, (Date.now() - launch.detectedAtMs) / 60_000);
  if (ageMin > 180) return null;

  return {
    id: `chain-${mint}`,
    mint,
    symbol,
    name: pair?.baseToken?.name ?? '',
    pairAddress: pair?.pairAddress ?? '',
    ageMinutes: Math.round(ageMin),
    volumeM5Usd: pair?.volume?.m5 ?? 0,
    volumeH1Usd: pair?.volume?.h1 ?? 0,
    buysM5: pair?.txns?.m5?.buys ?? 0,
    buysH1: pair?.txns?.h1?.buys ?? 0,
    liquidityUsd: pair?.liquidity?.usd ?? 0,
    marketCapUsd: pair?.marketCap ?? 0,
    graduationPct: Math.min(100, ((pair?.marketCap ?? 0) / GRADUATION_MCAP_USD) * 100),
    priceChangeM5Pct: pair?.priceChange?.m5 ?? 0,
    priceChangeH1Pct: pair?.priceChange?.h1 ?? 0,
    momentumScore: ageMin <= 15 ? 55 : 40,
    detectedAtMs: launch.detectedAtMs,
    dexUrl: pair?.url ?? `https://dexscreener.com/solana/${mint}`,
    pumpUrl: `https://pump.fun/coin/${mint}`,
    onChain: true,
    signature: launch.signature,
  };
}

function pumpScore(pair: DexPair, ageMin: number): number {
  const volM5 = pair.volume?.m5 ?? 0;
  const buysM5 = pair.txns?.m5?.buys ?? 0;
  const chgM5 = pair.priceChange?.m5 ?? 0;
  const grad = ((pair.marketCap ?? 0) / GRADUATION_MCAP_USD) * 100;

  let score = 0;
  if (ageMin <= 30) score += 30;
  else if (ageMin <= 120) score += 20;
  else if (ageMin <= 360) score += 10;

  if (volM5 >= 5_000) score += 15;
  if (volM5 >= 20_000) score += 10;
  if (buysM5 >= 8) score += 15;
  if (buysM5 >= 25) score += 10;
  if (chgM5 > 3) score += 10;
  if (grad >= 40 && grad < 95) score += 10;

  return Math.min(100, score);
}

function pairToPumpHit(pair: DexPair): PumpMomentumHit | null {
  if (!isPumpFunPair(pair)) return null;

  const mint = pair.baseToken?.address ?? '';
  if (!mint) return null;

  const ageMin = pairAgeMinutes(pair);
  if (ageMin > 720) return null;

  const volM5 = pair.volume?.m5 ?? 0;
  const buysM5 = pair.txns?.m5?.buys ?? 0;
  if (volM5 < 2_000 && buysM5 < 5) return null;

  const mcap = pair.marketCap ?? 0;
  const score = pumpScore(pair, ageMin);
  if (score < 35) return null;

  return {
    id: `${mint}-${pair.pairAddress ?? 'p'}`,
    mint,
    symbol: pair.baseToken?.symbol ?? mint.slice(0, 6),
    name: pair.baseToken?.name ?? '',
    pairAddress: pair.pairAddress ?? '',
    ageMinutes: Math.round(ageMin),
    volumeM5Usd: volM5,
    volumeH1Usd: pair.volume?.h1 ?? 0,
    buysM5,
    buysH1: pair.txns?.h1?.buys ?? 0,
    liquidityUsd: pair.liquidity?.usd ?? 0,
    marketCapUsd: mcap,
    graduationPct: Math.min(100, (mcap / GRADUATION_MCAP_USD) * 100),
    priceChangeM5Pct: pair.priceChange?.m5 ?? 0,
    priceChangeH1Pct: pair.priceChange?.h1 ?? 0,
    momentumScore: score,
    detectedAtMs: Date.now(),
    dexUrl: pair.url ?? `https://dexscreener.com/solana/${pair.pairAddress ?? mint}`,
    pumpUrl: `https://pump.fun/coin/${mint}`,
  };
}

/** Detect early pump.fun momentum via on-chain + DexScreener before UI trending. */
export async function runPumpFunScanner(): Promise<PumpScannerResponse> {
  const nowMs = Date.now();
  const heliusKey = process.env.HELIUS_API_KEY?.trim() ?? '';

  const [pumpSearch, profiles, onChain] = await Promise.all([
    searchDexPairs('pumpfun'),
    fetchLatestTokenProfiles(),
    heliusKey ? fetchOnChainLaunches(heliusKey) : Promise.resolve([]),
  ]);

  const pairs: DexPair[] = [...pumpSearch];
  const pairByMint = new Map<string, DexPair>();
  const mints = profiles.slice(0, 40).map((t) => t.tokenAddress).filter(Boolean) as string[];

  for (const launch of onChain) {
    if (!mints.includes(launch.mint)) mints.unshift(launch.mint);
  }

  for (const mint of mints.slice(0, 25)) {
    const tokenPairs = await fetchTokenPairs(mint);
    for (const p of tokenPairs) {
      if (isPumpFunPair(p)) {
        pairs.push(p);
        const addr = p.baseToken?.address ?? '';
        if (addr) pairByMint.set(addr, p);
      }
    }
  }

  const seen = new Set<string>();
  const hits: PumpMomentumHit[] = [];

  for (const launch of onChain) {
    const hit = onChainHitFromLaunch(launch, pairByMint.get(launch.mint));
    if (hit && !seen.has(launch.mint)) {
      seen.add(launch.mint);
      hits.push(hit);
    }
  }

  for (const pair of pairs) {
    const mint = pair.baseToken?.address ?? '';
    if (!mint || seen.has(mint)) continue;
    seen.add(mint);
    const hit = pairToPumpHit(pair);
    if (hit) hits.push(hit);
  }

  hits.sort((a, b) => b.momentumScore - a.momentumScore);

  const meta: ScannerMeta = {
    source: heliusKey ? 'helius-on-chain+dexscreener' : 'dexscreener',
    degraded: !heliusKey,
    message: heliusKey
      ? `On-chain Create events from program ${PUMP_PROGRAM_ID.slice(0, 8)}… merged with DexScreener volume.`
      : 'Set HELIUS_API_KEY for sub-minute on-chain Create detection. DexScreener still catches early volume.',
    apiKeys: { solscan: false, helius: !!heliusKey },
    polledAtMs: nowMs,
  };

  return { hits: hits.slice(0, 25), meta };
}
