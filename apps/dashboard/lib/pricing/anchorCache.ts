/** In-memory USD anchors — populated by PriceBootstrap from DexScreener. */
const anchors = new Map<string, number>();

export function setAnchorPrices(map: Record<string, number>): void {
  anchors.clear();
  for (const [mint, price] of Object.entries(map)) {
    if (price > 0) anchors.set(mint, price);
  }
}

export function getAnchorPrice(mint: string): number | undefined {
  const p = anchors.get(mint);
  return p && p > 0 ? p : undefined;
}

export function getAllAnchorPrices(): Record<string, number> {
  const out: Record<string, number> = {};
  anchors.forEach((price, mint) => {
    out[mint] = price;
  });
  return out;
}
