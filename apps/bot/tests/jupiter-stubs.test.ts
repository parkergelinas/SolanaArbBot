import { describe, expect, it } from 'vitest';

import { loadEnv } from '../src/config/env.js';
import { JupiterTriggerClient } from '../src/jupiter/trigger.js';
import { JupiterRecurringClient } from '../src/jupiter/recurring.js';

describe('Jupiter Trigger/Recurring stubs', () => {
  it('trigger client rejects when disabled', async () => {
    const env = loadEnv();
    const client = new JupiterTriggerClient(env);
    expect(client.buildCreateUrl()).toContain('api.jup.ag/trigger');
    await expect(
      client.createOrder({
        inputMint: 'a',
        outputMint: 'b',
        amount: '1',
        kind: 'take_profit',
        triggerPriceUsd: 1,
      }),
    ).rejects.toThrow('trigger_api_disabled');
  });

  it('recurring client rejects when disabled', async () => {
    const env = loadEnv();
    const client = new JupiterRecurringClient(env);
    expect(client.buildCreateUrl()).toContain('api.jup.ag/recurring');
    await expect(
      client.createOrder({
        inputMint: 'a',
        outputMint: 'b',
        amountPerCycle: '1',
        frequency: 'daily',
      }),
    ).rejects.toThrow('recurring_api_disabled');
  });
});
