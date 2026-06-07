/**
 * Jupiter Aggregator v6 — browser-side client.
 * Fetches quotes and VersionedTransaction payloads for signing by the connected wallet.
 *
 * Only used from client components / hooks — never server-side, because the
 * wallet's private key lives in the browser extension and transaction signing
 * must be delegated to the wallet adapter.
 */

import { VersionedTransaction } from '@solana/web3.js';

const JUPITER_API = 'https://api.jup.ag/swap/v1';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface JupRouteStep {
  swapInfo: {
    ammKey: string;
    label: string;
    inputMint: string;
    outputMint: string;
    inAmount: string;
    outAmount: string;
    feeAmount: string;
    feeMint: string;
  };
  percent: number;
}

export interface JupQuote {
  inputMint: string;
  inAmount: string;
  outputMint: string;
  outAmount: string;
  otherAmountThreshold: string;
  swapMode: 'ExactIn' | 'ExactOut';
  slippageBps: number;
  priceImpactPct: string;
  routePlan: JupRouteStep[];
  contextSlot?: number;
  timeTaken?: number;
}

export interface JupSwapTxResponse {
  swapTransaction: string; // base64-encoded VersionedTransaction
  lastValidBlockHeight: number;
}

export interface QuoteParams {
  inputMint: string;
  outputMint: string;
  /** Atomic amount (lamports / token units). */
  amount: number;
  slippageBps?: number;
  /** ExactIn = spend exactly `amount` of input; ExactOut = receive exactly `amount` of output. */
  swapMode?: 'ExactIn' | 'ExactOut';
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Polyfill for AbortSignal.timeout (Chrome 103+, FF 100+, Safari 15.4+). */
function timeoutSignal(ms: number): AbortSignal {
  if (typeof AbortSignal.timeout === 'function') return AbortSignal.timeout(ms);
  const ctrl = new AbortController();
  setTimeout(() => ctrl.abort(new Error(`timeout after ${ms}ms`)), ms);
  return ctrl.signal;
}

function validateQuote(json: unknown): JupQuote {
  const r = json as Record<string, unknown>;
  if (typeof r.outAmount !== 'string' || !/^\d+$/.test(r.outAmount)) {
    throw new Error('Jupiter quote: outAmount is missing or not a numeric string');
  }
  if (typeof r.inAmount !== 'string' || !/^\d+$/.test(r.inAmount)) {
    throw new Error('Jupiter quote: inAmount is missing or not a numeric string');
  }
  if (Number(r.outAmount) < 0 || Number(r.inAmount) < 0) {
    throw new Error('Jupiter quote: negative amount in response');
  }
  return r as unknown as JupQuote;
}

function validateSwapTx(json: unknown): JupSwapTxResponse {
  const r = json as Record<string, unknown>;
  if (typeof r.swapTransaction !== 'string' || r.swapTransaction.length < 100) {
    throw new Error('Jupiter swap: swapTransaction is missing or suspiciously short');
  }
  return r as unknown as JupSwapTxResponse;
}

// ── Public API ────────────────────────────────────────────────────────────────

/** Fetch a Jupiter quote. Throws on HTTP error or invalid response shape. */
export async function getJupiterQuote(params: QuoteParams): Promise<JupQuote> {
  const url = new URL(`${JUPITER_API}/quote`);
  url.searchParams.set('inputMint', params.inputMint);
  url.searchParams.set('outputMint', params.outputMint);
  url.searchParams.set('amount', String(params.amount));
  url.searchParams.set('slippageBps', String(params.slippageBps ?? 50));
  url.searchParams.set('restrictIntermediateTokens', 'true');
  if (params.swapMode) url.searchParams.set('swapMode', params.swapMode);

  const resp = await fetch(url.toString(), {
    headers: { Accept: 'application/json' },
    signal: timeoutSignal(8_000),
  });

  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Jupiter quote HTTP ${resp.status}: ${body.slice(0, 200)}`);
  }

  return validateQuote(await resp.json());
}

/**
 * Exchange a quote for a VersionedTransaction payload.
 * `priorityFeeLamports` is added as a compute budget instruction by Jupiter;
 * pass 0 to let Jupiter choose the default (usually 0).
 */
export async function getJupiterSwapTx(
  quote: JupQuote,
  userPublicKey: string,
  priorityFeeLamports?: number,
): Promise<JupSwapTxResponse> {
  const body: Record<string, unknown> = {
    quoteResponse: quote,
    userPublicKey,
    wrapAndUnwrapSol: true,
    dynamicComputeUnitLimit: true,
  };
  if (priorityFeeLamports != null && priorityFeeLamports > 0) {
    body.prioritizationFeeLamports = priorityFeeLamports;
  }

  const resp = await fetch(`${JUPITER_API}/swap`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify(body),
    signal: timeoutSignal(10_000),
  });

  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(`Jupiter swap HTTP ${resp.status}: ${text.slice(0, 200)}`);
  }

  return validateSwapTx(await resp.json());
}

/** Decode a base64 Jupiter swap payload to a signable VersionedTransaction. */
export function deserializeJupiterTx(base64: string): VersionedTransaction {
  return VersionedTransaction.deserialize(Buffer.from(base64, 'base64'));
}

/** Convert atomic string amount to UI float. */
export function atomicToUi(amount: string, decimals: number): number {
  // Use Number arithmetic — sufficient precision for display (15 sig-figs).
  return Number(amount) / Math.pow(10, decimals);
}

/** Parse priceImpactPct which may arrive as string or number. */
export function parsePriceImpact(raw: string | number | undefined): number {
  if (raw === undefined) return 0;
  const n = typeof raw === 'number' ? raw : parseFloat(raw);
  return Number.isFinite(n) ? Math.abs(n) : 0;
}

/** Derive a human-readable route label from the first hop's label. */
export function quoteRouteLabel(quote: JupQuote): string {
  const hops = quote.routePlan ?? [];
  if (hops.length === 0) return 'Jupiter';
  const labels = hops.map((h) => h.swapInfo.label).filter(Boolean);
  if (labels.length === 0) return 'Jupiter';
  if (labels.length === 1) return labels[0]!;
  return `${labels[0]} + ${labels.length - 1} more`;
}
