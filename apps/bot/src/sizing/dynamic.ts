export interface SizingInput {
  baseAmountUi: number;
  spreadBps: number;
  liquidityUsd: number;
  volatilityPct: number;
  routeQualityScore: number;
  recentFailureRate: number;
  minAmountUi: number;
  maxAmountUi: number;
}

/** Adaptive trade size from spread, liquidity, volatility, route depth, failure rate. */
export function computeDynamicSize(input: SizingInput): number {
  let mult = 1;

  if (input.spreadBps >= 50) mult *= 1.25;
  else if (input.spreadBps >= 20) mult *= 1.1;
  else if (input.spreadBps < 8) mult *= 0.5;

  if (input.liquidityUsd >= 500_000) mult *= 1.15;
  else if (input.liquidityUsd < 50_000) mult *= 0.6;

  if (input.volatilityPct > 5) mult *= 0.7;
  else if (input.volatilityPct < 1) mult *= 1.05;

  mult *= 0.5 + input.routeQualityScore * 0.5;
  mult *= 1 - Math.min(0.5, input.recentFailureRate);

  const sized = input.baseAmountUi * mult;
  return Math.max(input.minAmountUi, Math.min(input.maxAmountUi, sized));
}
