import { BotEngine } from './app/engine.js';
import { logger } from './logger.js';
import { closeDb } from './db/sqlite.js';
async function main() {
    const engine = new BotEngine();
    const iterations = Number(process.env.BOT_MAX_ITERATIONS ?? '0');
    logger.info({
        paperMode: process.env.BOT_PAPER_MODE !== '0',
        pairSource: process.env.BOT_PAIR_SOURCE ?? (process.env.CMC_API_KEY ? 'cmc' : 'static'),
        pumpEdge: process.env.BOT_ENABLE_PUMP_EDGE !== '0',
        jito: process.env.JITO_ENABLED === '1',
        iterations: iterations || 'unlimited',
    }, 'bot: starting');
    // ── Graceful shutdown ────────────────────────────────────────────────────
    const shutdown = async (signal) => {
        logger.info({ signal }, 'bot: shutdown signal received');
        await engine.gracefulStop(15_000);
        closeDb();
        logger.info('bot: clean exit');
        process.exit(0);
    };
    process.once('SIGTERM', () => void shutdown('SIGTERM'));
    process.once('SIGINT', () => void shutdown('SIGINT'));
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
    }
    else {
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
