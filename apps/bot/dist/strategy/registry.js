export class StrategyRegistry {
    strategies = [];
    register(strategy) {
        this.strategies.push(strategy);
        return this;
    }
    list() {
        return this.strategies;
    }
    evaluateAll(state, ctx) {
        const decisions = [];
        for (const s of this.strategies) {
            const d = s.evaluate(state, ctx);
            if (d)
                decisions.push(d);
        }
        return decisions.sort((a, b) => b.netProfitUsd - a.netProfitUsd);
    }
    best(state, ctx) {
        const all = this.evaluateAll(state, ctx);
        const actionable = all.filter((d) => !d.rejectionReason);
        return actionable[0] ?? all[0] ?? null;
    }
}
