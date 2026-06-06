import type { BotEnv } from '../config/env.js';
import type {
  JupiterTokenMeta,
  PriceV3Response,
  SwapQuoteRequest,
  SwapQuoteResponse,
  SwapTransactionRequest,
  SwapTransactionResponse,
} from './types.js';

export class JupiterClient {
  constructor(private readonly env: BotEnv) {}

  private headers(): Record<string, string> {
    const h: Record<string, string> = { Accept: 'application/json' };
    if (this.env.jupiterApiKey) {
      h['x-api-key'] = this.env.jupiterApiKey;
    }
    return h;
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
    const resp = await fetch(url, { headers: this.headers() });
    if (!resp.ok) {
      const body = await resp.text().catch(() => '');
      throw new Error(`Jupiter quote HTTP ${resp.status}: ${body.slice(0, 200)}`);
    }
    return (await resp.json()) as SwapQuoteResponse;
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
    return (await resp.json()) as SwapTransactionResponse;
  }

  async getPrices(mints: string[]): Promise<PriceV3Response> {
    if (mints.length === 0) return {};
    const url = `${this.env.jupiterPriceUrl}?ids=${mints.join(',')}`;
    const resp = await fetch(url, { headers: this.headers() });
    if (!resp.ok) {
      throw new Error(`Jupiter price HTTP ${resp.status}`);
    }
    return (await resp.json()) as PriceV3Response;
  }

  async fetchVerifiedTokens(limit = 500): Promise<JupiterTokenMeta[]> {
    const url = `${this.env.jupiterTokensBase}/tag?query=verified&limit=${limit}`;
    const resp = await fetch(url, { headers: this.headers() });
    if (!resp.ok) {
      throw new Error(`Jupiter tokens HTTP ${resp.status}`);
    }
    return (await resp.json()) as JupiterTokenMeta[];
  }

  async fetchStrictTokens(limit = 500): Promise<JupiterTokenMeta[]> {
    const url = `${this.env.jupiterTokensBase}/tag?query=strict&limit=${limit}`;
    const resp = await fetch(url, { headers: this.headers() });
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
