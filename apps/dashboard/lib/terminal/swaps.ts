import { formatCompact } from '@/lib/format/numbers';
import { refPriceForMint } from '@/lib/terminal/candles';
import { SOL_MINT } from '@/lib/terminal/tokens';
import type { SwapEvent, TokenPrice } from '@/lib/stream/types';
import { amountToHuman } from '@/stores/marketStore';

export type SwapDirection = 'buy' | 'sell' | 'swap';

/** SOL in → buying alt; SOL out → selling alt. */
export function swapDirection(s: SwapEvent): SwapDirection {
  if (s.token_in === SOL_MINT || s.token_in.startsWith('So1111')) return 'buy';
  if (s.token_out === SOL_MINT || s.token_out.startsWith('So1111')) return 'sell';
  return 'swap';
}

export function formatSwapAmount(mint: string, raw: string): string {
  return formatCompact(amountToHuman(mint, raw));
}

/** Price impact vs reference, in bps. Null when not computable. */
export function swapPriceImpactBps(
  s: SwapEvent,
  prices: Record<string, TokenPrice>,
): number | null {
  const outHuman = amountToHuman(s.token_out, s.amount_out);
  if (outHuman <= 0) return null;

  const inHuman = amountToHuman(s.token_in, s.amount_in);
  const refIn = refPriceForMint(s.token_in, prices[s.token_in]?.price_usd);
  const refOut = refPriceForMint(s.token_out, prices[s.token_out]?.price_usd);
  if (refOut <= 0) return null;

  const usdIn = inHuman * refIn;
  const impliedOut = usdIn / outHuman;
  const impact = ((impliedOut - refOut) / refOut) * 10_000;
  if (!Number.isFinite(impact)) return null;
  return Math.round(impact * 10) / 10;
}

const DEX_LABEL: Record<string, string> = {
  raydium: 'RAY',
  orca: 'ORC',
  jupiter: 'JUP',
};

export function dexBadge(dex: string): string {
  return DEX_LABEL[dex] ?? dex.slice(0, 3).toUpperCase();
}
