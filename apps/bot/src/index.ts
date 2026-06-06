import { BotEngine } from './app/engine.js';

async function main(): Promise<void> {
  const engine = new BotEngine();
  const iterations = Number(process.env.BOT_MAX_ITERATIONS ?? '0');
  console.log('[bot] starting — Jupiter api.jup.ag Swap v1 + Price v3 + Tokens v2');
  console.log('[bot] paper mode:', process.env.BOT_PAPER_MODE !== '0');

  if (iterations > 0) {
    await engine.runLoop(iterations);
  } else {
    await engine.runLoop();
  }

  console.log('[bot] stats', engine.getStats());
  console.log('[bot] analytics', engine.getAnalytics());
}

main().catch((err) => {
  console.error('[bot] fatal', err);
  process.exit(1);
});
