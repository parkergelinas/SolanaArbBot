import { describe, expect, it } from 'vitest';

import { loadEnv } from '../src/config/env.js';
import { JupiterClient } from '../src/jupiter/client.js';

describe('JupiterClient', () => {
  it('builds api.jup.ag swap v1 quote URL', () => {
    const env = loadEnv();
    expect(env.jupiterSwapBase).toBe('https://api.jup.ag/swap/v1');
    expect(env.jupiterPriceUrl).toBe('https://api.jup.ag/price/v3');
    expect(env.jupiterTokensBase).toBe('https://api.jup.ag/tokens/v2');

    const client = new JupiterClient(env);
    const url = client.buildQuoteUrl({
      inputMint: 'So11111111111111111111111111111111111111112',
      outputMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      amount: '1000000000',
      slippageBps: 50,
    });
    expect(url).toContain('api.jup.ag/swap/v1/quote');
    expect(url).not.toContain('lite-api');
  });
});
