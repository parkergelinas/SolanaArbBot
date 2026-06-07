/**
 * Pre-launch validation tests.
 *
 * Each test verifies one item from the mainnet launch checklist.  These tests
 * run against the bot's own config-loading logic so failures catch real
 * misconfigurations before a live deployment.
 *
 * ALL tests in this file can run offline — no RPC calls are made.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';

// Dynamic import so each test can stub env vars before the module loads.
async function loadEnvModule() {
  return import('../src/config/env.js');
}

describe('pre-launch: paper mode gate', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('defaults to paper mode (BOT_PAPER_MODE unset)', async () => {
    vi.stubEnv('BOT_PAPER_MODE', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.paperMode).toBe(true);
  });

  it('BOT_PAPER_MODE=1 keeps paper mode on', async () => {
    vi.stubEnv('BOT_PAPER_MODE', '1');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.paperMode).toBe(true);
  });

  it('BOT_PAPER_MODE=0 disables paper mode (live opt-in)', async () => {
    vi.stubEnv('BOT_PAPER_MODE', '0');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.paperMode).toBe(false);
  });

  it('any non-zero value keeps paper mode on (safe default)', async () => {
    for (const val of ['true', 'yes', 'on', '2']) {
      vi.stubEnv('BOT_PAPER_MODE', val);
      vi.resetModules();
      const { loadEnv } = await loadEnvModule();
      const e = loadEnv();
      expect(e.paperMode).toBe(true);
    }
  });
});

describe('pre-launch: RPC endpoint configuration', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('defaults to mainnet public RPC when unset', async () => {
    vi.stubEnv('SOLANA_RPC_URL', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.rpcUrl).toContain('mainnet');
  });

  it('accepts a custom RPC URL', async () => {
    vi.stubEnv('SOLANA_RPC_URL', 'https://mainnet.helius-rpc.com/?api-key=test');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.rpcUrl).toBe('https://mainnet.helius-rpc.com/?api-key=test');
  });

  it('does not silently accept a devnet URL as mainnet', async () => {
    vi.stubEnv('SOLANA_RPC_URL', 'https://api.devnet.solana.com');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    // loadEnv itself does not validate — the guard layer does.
    // This test asserts the URL is faithfully read so the guard can catch it.
    expect(e.rpcUrl).toContain('devnet');
    // In a live deployment, wallet guard's require_network_match() would reject this.
  });
});

describe('pre-launch: Jito configuration', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('Jito defaults to disabled (paper mode safe)', async () => {
    vi.stubEnv('JITO_ENABLED', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.jitoEnabled).toBe(false);
  });

  it('JITO_ENABLED=1 activates Jito bundle submission', async () => {
    vi.stubEnv('JITO_ENABLED', '1');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.jitoEnabled).toBe(true);
  });

  it('jitoTipLamports parses correctly', async () => {
    vi.stubEnv('JITO_TIP_LAMPORTS', '50000');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.jitoTipLamports).toBe(50_000);
  });
});

describe('pre-launch: trade size and profit guard', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('default trade amount is 1 SOL', async () => {
    vi.stubEnv('BOT_TRADE_AMOUNT_UI', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.tradeAmountUi).toBe(1);
  });

  it('trade amount parses from env', async () => {
    vi.stubEnv('BOT_TRADE_AMOUNT_UI', '0.5');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.tradeAmountUi).toBe(0.5);
  });

  it('default min profit is 0.25 USD', async () => {
    vi.stubEnv('BOT_MIN_PROFIT_USD', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.minProfitUsd).toBe(0.25);
  });

  it('minProfitLamports defaults to 0 (use USD guard instead)', async () => {
    vi.stubEnv('MIN_PROFIT_LAMPORTS', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.minProfitLamports).toBe(0);
  });
});

describe('pre-launch: slippage guard', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('default slippage is 50 bps (0.5%)', async () => {
    vi.stubEnv('BOT_SLIPPAGE_BPS', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.slippageBps).toBe(50);
  });

  it('strict slippage parses from env', async () => {
    vi.stubEnv('BOT_SLIPPAGE_BPS', '25');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.slippageBps).toBe(25);
  });
});

describe('pre-launch: spread threshold gate', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('default spread threshold is 80 bps', async () => {
    vi.stubEnv('SPREAD_THRESHOLD_BPS', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.spreadThresholdBps).toBe(80);
  });

  it('spread threshold parses from env', async () => {
    vi.stubEnv('SPREAD_THRESHOLD_BPS', '40');
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.spreadThresholdBps).toBe(40);
  });
});

describe('pre-launch: scan configuration', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('scan interval defaults to 2000ms', async () => {
    vi.stubEnv('BOT_SCAN_INTERVAL_MS', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.scanIntervalMs).toBe(2000);
  });

  it('pair source defaults to static when no CMC key', async () => {
    vi.stubEnv('CMC_API_KEY', undefined);
    vi.stubEnv('COINMARKETCAP_API_KEY', undefined);
    vi.stubEnv('BOT_PAIR_SOURCE', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.pairSource).toBe('static');
  });
});

describe('pre-launch: monitoring and observability', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('monitor port defaults to 3333', async () => {
    vi.stubEnv('MONITOR_PORT', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.monitorPort).toBe(3333);
  });

  it('log level defaults to info', async () => {
    vi.stubEnv('LOG_LEVEL', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.logLevel).toBe('info');
  });

  it('SQLite path defaults to local trades.db', async () => {
    vi.stubEnv('SQLITE_PATH', undefined);
    const { loadEnv } = await loadEnvModule();
    const e = loadEnv();
    expect(e.sqlitePath).toBe('./trades.db');
  });
});
