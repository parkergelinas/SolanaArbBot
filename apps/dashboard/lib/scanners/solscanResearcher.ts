import {
  fetchLatestTokenProfiles,
  fetchTokenPairs,
  searchDexPairs,
  type DexPair,
} from './dexscreener';
import type { ScannerMeta, ShitcoinWhaleHit, SolscanResearcherResponse } from './types';

const SOL_MINT = 'So11111111111111111111111111111111111111112';
const MAX_MCAP_USD = 750_000;
const MIN_BUY_VOL_M5 = 8_000;
const MIN_BUYS_M5 = 12;
const SOLSCAN_BASE = 'https://solscan.io';

interface SolscanSwapRow {
  trans_id?: string;
  block_time?: number;
  from_address?: string;
  amount?: number;
  value?: number;
}

function solscanTxUrl(sig: string) {
  return `${SOLSCAN_BASE}/tx/${sig}`;
}

function solscanWalletUrl(wallet: string) {
  return `${SOLSCAN_BASE}/account/${wallet}`;
}

function solscanTokenUrl(mint: string) {
  return `${SOLSCAN_BASE}/token/${mint}`;
}

function scoreShitcoinHit(pair: DexPair): number {
  const mcap = pair.marketCap ?? 0;
  const liq = pair.liquidity?.usd ?? 0;
  const volM5 = pair.volume?.m5 ?? 0;
  const buysM5 = pair.txns?.m5?.buys ?? 0;
  const chgM5 = pair.priceChange?.m5 ?? 0;

  let score = 0;
  if (mcap > 0 && mcap < MAX_MCAP_USD) score += 25;
  if (liq > 0 && liq < 80_000) score += 15;
  if (volM5 >= MIN_BUY_VOL_M5) score += 20;
  if (buysM5 >= MIN_BUYS_M5) score += 20;
  if (chgM5 > 5) score += 10;
  if (pairAgeMinutes(pair) < 120) score += 10;
  return Math.min(100, score);
}

function pairAgeMinutes(pair: DexPair, nowMs = Date.now()): number {
  const created = pair.pairCreatedAt ?? 0;
  if (!created) return 9999;
  return Math.max(0, (nowMs - created) / 60_000);
}

function pairToHit(
  pair: DexPair,
  wallet: string,
  source: ShitcoinWhaleHit['source'],
  signature?: string,
): ShitcoinWhaleHit | null {
  const mint = pair.baseToken?.address;
  if (!mint || mint === SOL_MINT) return null;

  const mcap = pair.marketCap ?? 0;
  const volM5 = pair.volume?.m5 ?? 0;
  const buysM5 = pair.txns?.m5?.buys ?? 0;
  if (mcap > MAX_MCAP_USD || volM5 < MIN_BUY_VOL_M5 || buysM5 < MIN_BUYS_M5) return null;

  const score = scoreShitcoinHit(pair);
  if (score < 40) return null;

  const priceUsd = Number(pair.priceUsd ?? 0);
  const estSol = volM5 > 0 ? volM5 / 150 : 0;

  return {
    id: `${mint}-${wallet.slice(0, 8)}`,
    wallet,
    tokenMint: mint,
    tokenSymbol: pair.baseToken?.symbol ?? mint.slice(0, 6),
    amountSol: estSol,
    amountUsd: volM5,
    marketCapUsd: mcap,
    liquidityUsd: pair.liquidity?.usd ?? 0,
    detectedAtMs: Date.now(),
    signature,
    source,
    score,
    solscanUrl: signature ? solscanTxUrl(signature) : solscanWalletUrl(wallet),
    tokenUrl: solscanTokenUrl(mint),
  };
}

async function fetchSolscanSwaps(
  mint: string,
  apiKey: string,
): Promise<SolscanSwapRow[]> {
  const url = `https://pro-api.solscan.io/v2.0/token/defi/activities?address=${mint}&activity_type=ACTIVITY_TOKEN_SWAP&page=1&page_size=20`;
  const res = await fetch(url, {
    headers: { token: apiKey, Accept: 'application/json' },
    next: { revalidate: 0 },
  });
  if (!res.ok) return [];
  const body = (await res.json()) as { data?: SolscanSwapRow[] };
  return body.data ?? [];
}

async function fetchHeliusLargeBuyers(
  mint: string,
  apiKey: string,
): Promise<{ wallet: string; signature: string; amountUsd: number }[]> {
  const url = `https://api.helius.xyz/v0/addresses/${mint}/transactions?api-key=${apiKey}&limit=15`;
  const res = await fetch(url, { next: { revalidate: 0 } });
  if (!res.ok) return [];

  const txs = (await res.json()) as {
    signature?: string;
    feePayer?: string;
    tokenTransfers?: { mint?: string; tokenAmount?: number }[];
    nativeTransfers?: { amount?: number }[];
  }[];

  const out: { wallet: string; signature: string; amountUsd: number }[] = [];
  for (const tx of txs) {
    const wallet = tx.feePayer ?? '';
    const sig = tx.signature ?? '';
    if (!wallet || !sig) continue;
    const solLamports =
      tx.nativeTransfers?.reduce((s, t) => s + (t.amount ?? 0), 0) ?? 0;
    const amountUsd = (solLamports / 1e9) * 150;
    if (amountUsd >= 500) {
      out.push({ wallet, signature: sig, amountUsd });
    }
  }
  return out;
}

/** Scan DexScreener + optional Solscan/Helius for shitcoin whale wallets. */
export async function runSolscanResearcher(): Promise<SolscanResearcherResponse> {
  const solscanKey = process.env.SOLSCAN_API_KEY?.trim() ?? '';
  const heliusKey = process.env.HELIUS_API_KEY?.trim() ?? '';
  const nowMs = Date.now();

  const [boosted, profiles] = await Promise.all([
    searchDexPairs('solana meme'),
    fetchLatestTokenProfiles(),
  ]);

  const candidateMints = new Set<string>();
  for (const p of boosted.slice(0, 40)) {
    const mint = p.baseToken?.address;
    if (mint && mint !== SOL_MINT) candidateMints.add(mint);
  }
  for (const t of profiles.slice(0, 30)) {
    if (t.tokenAddress) candidateMints.add(t.tokenAddress);
  }

  const hits: ShitcoinWhaleHit[] = [];
  const seen = new Set<string>();

  for (const mint of Array.from(candidateMints).slice(0, 25)) {
    const pairs = await fetchTokenPairs(mint);
    const pair = pairs[0];
    if (!pair) continue;

    let wallets: { wallet: string; signature?: string; source: ShitcoinWhaleHit['source'] }[] =
      [];

    if (solscanKey) {
      const swaps = await fetchSolscanSwaps(mint, solscanKey);
      for (const s of swaps) {
        const w = s.from_address ?? '';
        if (w) wallets.push({ wallet: w, signature: s.trans_id, source: 'solscan' });
      }
    } else if (heliusKey) {
      const buyers = await fetchHeliusLargeBuyers(mint, heliusKey);
      for (const b of buyers) {
        wallets.push({ wallet: b.wallet, signature: b.signature, source: 'helius' });
      }
    } else {
      wallets.push({
        wallet: 'unknown — connect SOLSCAN_API_KEY or HELIUS_API_KEY',
        source: 'dexscreener',
      });
    }

    for (const w of wallets.slice(0, 3)) {
      const key = `${mint}:${w.wallet}`;
      if (seen.has(key)) continue;
      seen.add(key);

      const hit = pairToHit(pair, w.wallet, w.source, w.signature);
      if (hit) hits.push(hit);
    }
  }

  hits.sort((a, b) => b.score - a.score);

  const degraded = !solscanKey && !heliusKey;
  const meta: ScannerMeta = {
    source: degraded ? 'dexscreener-inferred' : solscanKey ? 'solscan+dex' : 'helius+dex',
    degraded,
    message: degraded
      ? 'Wallet addresses require SOLSCAN_API_KEY or HELIUS_API_KEY. Token momentum still live via DexScreener.'
      : undefined,
    apiKeys: { solscan: !!solscanKey, helius: !!heliusKey },
    polledAtMs: nowMs,
  };

  return { hits: hits.slice(0, 30), meta };
}
