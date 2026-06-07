import type { BotEnv } from '../config/env.js';
import type {
  JupiterTokenMeta,
  PriceV3Response,
  SwapQuoteRequest,
  SwapQuoteResponse,
  SwapTransactionRequest,
  SwapTransactionResponse,
} from './types.js';

// ── SEC-3: Runtime response validators ───────────────────────────────────────
// Jupiter returns JSON whose types are not verified at the network boundary.
// A compromised/misconfigured endpoint could return outAmount: -1 or a truncated
// swapTransaction; these guards abort before poisoned data reaches execution logic.

function validateQuoteResponse(json: unknown, label: string): SwapQuoteResponse {
  const r = json as Record<string, unknown>;
  if (typeof r.outAmount !== 'string' || !/^\d+$/.test(r.outAmount)) {
    throw new Error(`Jupiter quote (${label}): outAmount is missing or not a numeric string`);
  }
  if (typeof r.inAmount !== 'string') {
    throw new Error(`Jupiter quote (${label}): inAmount is missing`);
  }
  if (Number(r.outAmount) < 0 || Number(r.inAmount) < 0) {
    throw new Error(`Jupiter quote (${label}): negative amount received`);
  }
  return r as unknown as SwapQuoteResponse;
}

function validateSwapResponse(json: unknown): SwapTransactionResponse {
  const r = json as Record<string, unknown>;
  if (typeof r.swapTransaction !== 'string' || r.swapTransaction.length < 100) {
    throw new Error('Jupiter swap: swapTransaction field missing or suspiciously short');
  }
  return r as unknown as SwapTransactionResponse;
}

export class JupiterClient {
  private _429BackoffUntil = 0; // epoch ms — shared across all calls on this client

  constructor(private readonly env: BotEnv) {}

  private headers(): Record<string, string> {
    const h: Record<string, string> = { Accept: 'application/json' };
    if (this.env.jupiterApiKey) {
      h['x-api-key'] = this.env.jupiterApiKey;
    }
    return h;
  }

  /** Throws `RateLimited` error class when 429 backoff is active. */
  private async fetchWithBackoff(url: string, init?: RequestInit): Promise<Response> {
    const now = Date.now();
    if (now < this._429BackoffUntil) {
      throw new Error(`Jupiter rate-limited — retry after ${Math.ceil((this._429BackoffUntil - now) / 1000)}s`);
    }
    const resp = await fetch(url, init);
    if (resp.status === 429) {
      // Back off 20s + up to 10s jitter before retrying
      this._429BackoffUntil = Date.now() + 20_000 + Math.random() * 10_000;
      const body = await resp.text().catch(() => '');
      throw new Error(`Jupiter quote HTTP 429: ${body.slice(0, 120)}`);
    }
    return resp;
  }

  buildQuoteUrl(req: SwapQuoteRequest): string {
    const params = new URLSearchParams({
      inputMint: req.inputMint,
      outputMint: req.outputMint,
      amount: String(req.amount),
      slippageBps: String(req.slippageBps ?? 50),
      restrictIntermediateTokens: String(req.restrictIntermediateTokens ?? false),
    });
    if (req.swapMode) params.set('swapMode', req.swapMode);
    return `${this.env.jupiterSwapBase}/quote?${params}`;
  }

  async getQuote(req: SwapQuoteRequest): Promise<SwapQuoteResponse> {
    const url = this.buildQuoteUrl(req);
    const resp = await this.fetchWithBackoff(url, { headers: this.headers() });
    if (!resp.ok) {
      const body = await resp.text().catch(() => '');
      throw new Error(`Jupiter quote HTTP ${resp.status}: ${body.slice(0, 200)}`);
    }
    const json = await resp.json();
    return validateQuoteResponse(json, `${req.inputMint}→${req.outputMint}`);
  }

  async getSwapTransaction(req: SwapTransactionRequest): Promise<SwapTransactionResponse> {
    const resp = await fetch(`${this.env.jupiterSwapBase}/swap`, {
      method: 'POST',
      headers: { ...this.headers(), 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!resp.ok) {
      const body = await resp.text().catch(() => '');
      throw new Error(`Jupiter swap HTTP ${resp.status}: ${body.slice(0, 200)}`);
    }
    const json = await resp.json();
    return validateSwapResponse(json);
  }

  async getPrices(mints: string[]): Promise<PriceV3Response> {
    if (mints.length === 0) return {};
    const url = `${this.env.jupiterPriceUrl}?ids=${mints.join(',')}`;
    const resp = await this.fetchWithBackoff(url, { headers: this.headers() });
    if (!resp.ok) {
      throw new Error(`Jupiter price HTTP ${resp.status}`);
    }
    return (await resp.json()) as PriceV3Response;
  }

  async fetchVerifiedTokens(limit = 500): Promise<JupiterTokenMeta[]> {
    const url = `${this.env.jupiterTokensBase}/tag?query=verified&limit=${limit}`;
    // Use fetchWithBackoff so tokens-API 429s share the same backoff state as
    // quote/price calls — prevents hammering after the public rate limit fires.
    const resp = await this.fetchWithBackoff(url, { headers: this.headers() });
    if (!resp.ok) {
      throw new Error(`Jupiter tokens HTTP ${resp.status}`);
    }
    return (await resp.json()) as JupiterTokenMeta[];
  }

  async fetchStrictTokens(limit = 500): Promise<JupiterTokenMeta[]> {
    const url = `${this.env.jupiterTokensBase}/tag?query=strict&limit=${limit}`;
    const resp = await this.fetchWithBackoff(url, { headers: this.headers() });
    if (!resp.ok) {
      throw new Error(`Jupiter strict tokens HTTP ${resp.status}`);
    }
    return (await resp.json()) as JupiterTokenMeta[];
  }
}

/** Parse atomic string amount to UI float using decimals. */
export function atomicToUi(amount: string, decimals: number): number {
  const n = BigInt(amount);
  const scale = 10n ** BigInt(decimals);
  const whole = n / scale;
  const frac = n % scale;
  return Number(whole) + Number(frac) / Number(scale);
}

export function uiToAtomic(amountUi: number, decimals: number): bigint {
  const scale = 10 ** decimals;
  return BigInt(Math.floor(amountUi * scale));
}

export function parsePriceImpactPct(raw: string | number | undefined): number {
  if (raw === undefined) return 0;
  const n = typeof raw === 'number' ? raw : Number.parseFloat(raw);
  return Number.isFinite(n) ? Math.abs(n) : 0;
}

export function routeHopCount(quote: SwapQuoteResponse): number {
  return quote.routePlan?.length ?? 0;
}
