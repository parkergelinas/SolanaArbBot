/** Jupiter Trigger API client stub — TP/SL/breakout orders (not wired to engine yet). */
/**
 * Stub client for Jupiter Trigger API.
 * Live wiring deferred — enable via `BOT_ENABLE_TRIGGER_API=1` when ready.
 */
export class JupiterTriggerClient {
    env;
    baseUrl;
    constructor(env, baseUrl = 'https://api.jup.ag/trigger/v1') {
        this.env = env;
        this.baseUrl = baseUrl;
    }
    headers() {
        const h = { Accept: 'application/json' };
        if (this.env.jupiterApiKey)
            h['x-api-key'] = this.env.jupiterApiKey;
        return h;
    }
    /** Create a trigger order — throws until live integration is implemented. */
    async createOrder(_req) {
        if (!this.env.enableTriggerApi) {
            throw new Error('trigger_api_disabled');
        }
        throw new Error('trigger_api_not_wired');
    }
    /** Cancel an existing trigger order — stub. */
    async cancelOrder(_orderId) {
        if (!this.env.enableTriggerApi) {
            throw new Error('trigger_api_disabled');
        }
        throw new Error('trigger_api_not_wired');
    }
    buildCreateUrl() {
        return `${this.baseUrl}/createOrder`;
    }
}
