/**
 * CLI entry point for the multi-strategy backtest runner.
 *
 * Environment variables:
 *   BACKTEST_DURATION_HR=3     — hours per strategy window (default 0.33 = 20 min)
 *   BACKTEST_MIN_PROFIT=0.05   — min net profit USD threshold
 *   BOT_SOL_PRICE=150          — SOL/USD price for notional calculations
 *   BACKTEST_STOP_ON_PASS=1    — stop after first profitable strategy
 *   BACKTEST_OUT_DIR=./data    — output directory for the JSON report
 *
 * Run:
 *   BOT_PAPER_MODE=1 BOT_LIVE_QUOTES=1 \
 *   BACKTEST_DURATION_HR=3 \
 *   npx tsx src/backtest/run-backtest.ts
 */

import { runMultiStrategyBacktest } from './multi-strategy-runner.js';

const durationHr = Number(process.env['BACKTEST_DURATION_HR'] ?? '0.33');
const stopOnFirstPass = process.env['BACKTEST_STOP_ON_PASS'] === '1';
const minProfitUsd = Number(process.env['BACKTEST_MIN_PROFIT'] ?? '0.01');
const solPriceUsd  = Number(process.env['BOT_SOL_PRICE'] ?? '150');
const outputDir    = process.env['BACKTEST_OUT_DIR'];

console.log('SolanaArbBot — Multi-Strategy Live-Quote Backtest');
console.log('=================================================');
console.log(`Duration per strategy : ${durationHr} hr (${Math.round(durationHr * 60)} min)`);
console.log(`Stop on first pass    : ${stopOnFirstPass}`);
console.log(`Min profit threshold  : $${minProfitUsd}`);
console.log(`SOL price             : $${solPriceUsd}`);
console.log('');

runMultiStrategyBacktest({
  durationHrPerStrategy: durationHr,
  minProfitUsd,
  solPriceUsd,
  outputDir,
  stopOnFirstPass,
})
  .then((report) => {
    const passed = report.passedStrategies.length;
    const total  = report.totalStrategiesTested;
    console.log(`\nBacktest complete. ${passed}/${total} strategies passed.`);
    process.exit(passed > 0 ? 0 : 1);
  })
  .catch((err) => {
    console.error('Backtest runner fatal error:', err);
    process.exit(2);
  });
