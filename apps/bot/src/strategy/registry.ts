import type { MarketState } from '../market/state.js';
import type { Strategy, StrategyContext, TradeDecision } from './types.js';

export class StrategyRegistry {
  private strategies: Strategy[] = [];

  register(strategy: Strategy): this {
    this.strategies.push(strategy);
    return this;
  }

  list(): readonly Strategy[] {
    return this.strategies;
  }

  evaluateAll(state: MarketState, ctx: StrategyContext): TradeDecision[] {
    const decisions: TradeDecision[] = [];
    for (const s of this.strategies) {
      const d = s.evaluate(state, ctx);
      if (d) decisions.push(d);
    }
    return decisions.sort((a, b) => b.netProfitUsd - a.netProfitUsd);
  }

  best(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const all = this.evaluateAll(state, ctx);
    const actionable = all.filter((d) => !d.rejectionReason);
    return actionable[0] ?? all[0] ?? null;
  }
}
