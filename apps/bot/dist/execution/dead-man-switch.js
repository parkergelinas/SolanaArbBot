import { EventEmitter } from 'events';
import { logger } from '../logger.js';
/**
 * Halts all trading after N consecutive transaction failures.
 * Emits a 'halt' event and refuses further executions until reset().
 */
export class DeadManSwitch extends EventEmitter {
    maxFailures;
    consecutiveFailures = 0;
    halted = false;
    haltEvent = null;
    constructor(opts = {}) {
        super();
        this.maxFailures = opts.maxConsecutiveFailures ?? 3;
    }
    isHalted() {
        return this.halted;
    }
    getHaltEvent() {
        return this.haltEvent;
    }
    getConsecutiveFailures() {
        return this.consecutiveFailures;
    }
    recordSuccess() {
        this.consecutiveFailures = 0;
    }
    recordFailure(reason) {
        if (this.halted)
            return;
        this.consecutiveFailures += 1;
        logger.warn({ consecutiveFailures: this.consecutiveFailures, maxFailures: this.maxFailures, reason }, 'dead-man-switch: transaction failure recorded');
        if (this.consecutiveFailures >= this.maxFailures) {
            this.halted = true;
            this.haltEvent = {
                reason: `${this.maxFailures} consecutive failures — last: ${reason}`,
                consecutiveFailures: this.consecutiveFailures,
                haltedAtMs: Date.now(),
            };
            logger.error(this.haltEvent, 'dead-man-switch: HALTED — trading suspended');
            this.emit('halt', this.haltEvent);
        }
    }
    /** Re-enable trading after operator review. */
    reset() {
        this.consecutiveFailures = 0;
        this.halted = false;
        this.haltEvent = null;
        logger.info('dead-man-switch: reset by operator');
    }
}
