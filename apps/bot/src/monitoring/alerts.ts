/**
 * Telegram and Discord webhook alert delivery.
 * Fires on: PnL dropping below threshold, bot halt, custom messages.
 */

import { logger } from '../logger.js';

const TELEGRAM_TOKEN = process.env.TELEGRAM_BOT_TOKEN;
const TELEGRAM_CHAT_ID = process.env.TELEGRAM_CHAT_ID;
const DISCORD_WEBHOOK_URL = process.env.DISCORD_WEBHOOK_URL;

export async function sendAlert(message: string): Promise<void> {
  const results = await Promise.allSettled([
    sendTelegram(message),
    sendDiscord(message),
  ]);
  for (const r of results) {
    if (r.status === 'rejected') {
      logger.warn({ err: r.reason }, 'alert delivery failed');
    }
  }
}

async function sendTelegram(message: string): Promise<void> {
  if (!TELEGRAM_TOKEN || !TELEGRAM_CHAT_ID) return;

  const url = `https://api.telegram.org/bot${TELEGRAM_TOKEN}/sendMessage`;
  const resp = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      chat_id: TELEGRAM_CHAT_ID,
      text: `🤖 SolanaArbBot\n${message}`,
      parse_mode: 'Markdown',
    }),
    signal: AbortSignal.timeout(5_000),
  });

  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Telegram HTTP ${resp.status}: ${body.slice(0, 200)}`);
  }
}

async function sendDiscord(message: string): Promise<void> {
  if (!DISCORD_WEBHOOK_URL) return;

  const resp = await fetch(DISCORD_WEBHOOK_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content: `**SolanaArbBot** ${message}` }),
    signal: AbortSignal.timeout(5_000),
  });

  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Discord HTTP ${resp.status}: ${body.slice(0, 200)}`);
  }
}

/**
 * Check PnL and send alert if it falls below the configured threshold.
 * Call this after every executed trade.
 */
export async function checkPnlAlert(
  totalPnlUsd: number,
  solPriceUsd: number,
): Promise<void> {
  const thresholdSol = Number(process.env.ALERT_PNL_THRESHOLD_SOL ?? '-0.1');
  const thresholdUsd = thresholdSol * solPriceUsd;

  if (totalPnlUsd < thresholdUsd) {
    await sendAlert(
      `⚠️ *PnL alert*: session PnL is *${totalPnlUsd.toFixed(4)} USD* (threshold ${thresholdUsd.toFixed(4)} USD)`,
    );
  }
}

export async function alertHalt(reason: string): Promise<void> {
  await sendAlert(`🛑 *Bot halted*: ${reason}`);
}
