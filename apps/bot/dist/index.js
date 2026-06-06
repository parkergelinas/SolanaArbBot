import { BotEngine } from './app/engine.js';
async function main() {
    const engine = new BotEngine();
    const iterations = Number(process.env.BOT_MAX_ITERATIONS ?? '0');
    console.log('[bot] starting — Jupiter api.jup.ag + CMC top-100 pairs + Pump.fun edge');
    console.log('[bot] paper mode:', process.env.BOT_PAPER_MODE !== '0');
    console.log('[bot] pair source:', process.env.BOT_PAIR_SOURCE ?? (process.env.CMC_API_KEY ? 'cmc' : 'static'));
    console.log('[bot] pump edge:', process.env.BOT_ENABLE_PUMP_EDGE !== '0');
    if (iterations > 0) {
        await engine.runLoop(iterations);
    }
    else {
        await engine.runLoop();
    }
    console.log('[bot] stats', engine.getStats());
    console.log('[bot] analytics', engine.getAnalytics());
}
main().catch((err) => {
    console.error('[bot] fatal', err);
    process.exit(1);
});
