import { describe, expect, it, vi, afterEach } from 'vitest';

describe('env config', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('demo is opt-in only', async () => {
    vi.stubEnv('NEXT_PUBLIC_TERMINAL_DEMO', undefined);
    const { isTerminalDemoEnabled } = await import('@/lib/config/env');
    expect(isTerminalDemoEnabled()).toBe(false);

    vi.stubEnv('NEXT_PUBLIC_TERMINAL_DEMO', '0');
    expect(isTerminalDemoEnabled()).toBe(false);

    vi.stubEnv('NEXT_PUBLIC_TERMINAL_DEMO', '1');
    expect(isTerminalDemoEnabled()).toBe(true);
  });

  it('defaults solana network to devnet when unset', async () => {
    vi.stubEnv('NEXT_PUBLIC_SOLANA_NETWORK', undefined);
    const { defaultSolanaNetwork } = await import('@/lib/config/env');
    expect(defaultSolanaNetwork()).toBe('devnet');
  });
});
