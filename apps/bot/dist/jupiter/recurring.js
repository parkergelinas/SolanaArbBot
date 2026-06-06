/** Jupiter Recurring API client stub — DCA / treasury automation (not wired yet). */
/**
 * Stub client for Jupiter Recurring API.
 * Live wiring deferred — enable via `BOT_ENABLE_RECURRING_API=1` when ready.
 */
export class JupiterRecurringClient {
    env;
    baseUrl;
    constructor(env, baseUrl = 'https://api.jup.ag/recurring/v1') {
        this.env = env;
        this.baseUrl = baseUrl;
    }
    headers() {
        const h = { Accept: 'application/json' };
        if (this.env.jupiterApiKey)
            h['x-api-key'] = this.env.jupiterApiKey;
        return h;
    }
    async createOrder(_req) {
        if (!this.env.enableRecurringApi) {
            throw new Error('recurring_api_disabled');
        }
        throw new Error('recurring_api_not_wired');
    }
    async pauseOrder(_orderId) {
        if (!this.env.enableRecurringApi) {
            throw new Error('recurring_api_disabled');
        }
        throw new Error('recurring_api_not_wired');
    }
    buildCreateUrl() {
        return `${this.baseUrl}/createOrder`;
    }
}
