/** Jupiter Trigger API client stub — TP/SL/breakout orders (not wired to engine yet). */

import type { BotEnv } from '../config/env.js';

export type TriggerOrderKind = 'take_profit' | 'stop_loss' | 'breakout';

export interface TriggerOrderRequest {
  inputMint: string;
  outputMint: string;
  amount: string;
  kind: TriggerOrderKind;
  triggerPriceUsd: number;
  slippageBps?: number;
}

export interface TriggerOrderResponse {
  orderId: string;
  status: 'pending' | 'active';
}

/**
 * Stub client for Jupiter Trigger API.
 * Live wiring deferred — enable via `BOT_ENABLE_TRIGGER_API=1` when ready.
 */
export class JupiterTriggerClient {
  constructor(
    private readonly env: BotEnv,
    private readonly baseUrl = 'https://api.jup.ag/trigger/v1',
  ) {}

  private headers(): Record<string, string> {
    const h: Record<string, string> = { Accept: 'application/json' };
    if (this.env.jupiterApiKey) h['x-api-key'] = this.env.jupiterApiKey;
    return h;
  }

  /** Create a trigger order — throws until live integration is implemented. */
  async createOrder(_req: TriggerOrderRequest): Promise<TriggerOrderResponse> {
    if (!this.env.enableTriggerApi) {
      throw new Error('trigger_api_disabled');
    }
    throw new Error('trigger_api_not_wired');
  }

  /** Cancel an existing trigger order — stub. */
  async cancelOrder(_orderId: string): Promise<void> {
    if (!this.env.enableTriggerApi) {
      throw new Error('trigger_api_disabled');
    }
    throw new Error('trigger_api_not_wired');
  }

  buildCreateUrl(): string {
    return `${this.baseUrl}/createOrder`;
  }
}
