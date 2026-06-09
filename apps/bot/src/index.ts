import { BotEngine } from './app/engine.js';
import { logger } from './logger.js';
import { closeDb } from './db/sqlite.js';

async function main(): Promise<void> {
  // ── Scanner mode ─────────────────────────────────────────────────────────
  if (process.env.SCANNER_MODE === '1') {
    process.env.BOT_PAPER_MODE = '1';
    process.env.BOT_LIVE_QUOTES = '1';
    const { startScannerDaemon } = await import('./scanner/live-collector.js');
    const daemon = await startScannerDaemon();
    const shutdown = async (signal: string): Promise<void> => {
      logger.info({ signal }, 'scanner: shutdown signal received');
      await daemon.stop();
      logger.info('scanner: clean exit');
      process.exit(0);
    };
    process.once('SIGTERM', () => void shutdown('SIGTERM'));
    process.once('SIGINT',  () => void shutdown('SIGINT'));
    return; // Express server keeps the event loop alive; signals trigger shutdown above
  }

  const engine = new BotEngine();
  const iterations = Number(process.env.BOT_MAX_ITERATIONS ?? '0');

  logger.info(
    {
      paperMode: process.env.BOT_PAPER_MODE !== '0',
      pairSource: process.env.BOT_PAIR_SOURCE ?? (process.env.CMC_API_KEY ? 'cmc' : 'static'),
      pumpEdge: process.env.BOT_ENABLE_PUMP_EDGE !== '0',
      jito: process.env.JITO_ENABLED === '1',
      iterations: iterations || 'unlimited',
    },
    'bot: starting',
  );

  // ── Graceful shutdown ────────────────────────────────────────────────────
  const shutdown = async (signal: string): Promise<void> => {
    logger.info({ signal }, 'bot: shutdown signal received');
    await engine.gracefulStop(15_000);
    closeDb();
    logger.info('bot: clean exit');
    process.exit(0);
  };

  process.once('SIGTERM', () => void shutdown('SIGTERM'));
  process.once('SIGINT',  () => void shutdown('SIGINT'));

  process.on('uncaughtException', (err) => {
    logger.fatal({ err }, 'bot: uncaught exception');
    process.exit(1);
  });

  process.on('unhandledRejection', (reason) => {
    logger.error({ reason }, 'bot: unhandled promise rejection');
  });

  // ── Run ──────────────────────────────────────────────────────────────────
  if (iterations > 0) {
    await engine.runLoop(iterations);
  } else {
    await engine.runLoop();
  }

  logger.info(engine.getStats(), 'bot: stats');
  logger.info(engine.getAnalytics(), 'bot: analytics');
  closeDb();
}

main().catch((err) => {
  logger.fatal({ err }, 'bot: fatal startup error');
  process.exit(1);
});
