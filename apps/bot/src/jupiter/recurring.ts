/** Jupiter Recurring API client stub — DCA / treasury automation (not wired yet). */

import type { BotEnv } from '../config/env.js';

export type RecurringFrequency = 'hourly' | 'daily' | 'weekly';

export interface RecurringOrderRequest {
  inputMint: string;
  outputMint: string;
  amountPerCycle: string;
  frequency: RecurringFrequency;
  slippageBps?: number;
}

export interface RecurringOrderResponse {
  orderId: string;
  status: 'active' | 'paused';
}

/**
 * Stub client for Jupiter Recurring API.
 * Live wiring deferred — enable via `BOT_ENABLE_RECURRING_API=1` when ready.
 */
export class JupiterRecurringClient {
  constructor(
    private readonly env: BotEnv,
    private readonly baseUrl = 'https://api.jup.ag/recurring/v1',
  ) {}

  private headers(): Record<string, string> {
    const h: Record<string, string> = { Accept: 'application/json' };
    if (this.env.jupiterApiKey) h['x-api-key'] = this.env.jupiterApiKey;
    return h;
  }

  async createOrder(_req: RecurringOrderRequest): Promise<RecurringOrderResponse> {
    if (!this.env.enableRecurringApi) {
      throw new Error('recurring_api_disabled');
    }
    throw new Error('recurring_api_not_wired');
  }

  async pauseOrder(_orderId: string): Promise<void> {
    if (!this.env.enableRecurringApi) {
      throw new Error('recurring_api_disabled');
    }
    throw new Error('recurring_api_not_wired');
  }

  buildCreateUrl(): string {
    return `${this.baseUrl}/createOrder`;
  }
}
