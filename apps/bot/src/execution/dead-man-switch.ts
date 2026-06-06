import { EventEmitter } from 'events';
import { logger } from '../logger.js';

export interface DeadManSwitchOptions {
  /** Number of consecutive failures before halting. Default 3. */
  maxConsecutiveFailures?: number;
}

export interface HaltEvent {
  reason: string;
  consecutiveFailures: number;
  haltedAtMs: number;
}

/**
 * Halts all trading after N consecutive transaction failures.
 * Emits a 'halt' event and refuses further executions until reset().
 */
export class DeadManSwitch extends EventEmitter {
  private readonly maxFailures: number;
  private consecutiveFailures = 0;
  private halted = false;
  private haltEvent: HaltEvent | null = null;

  constructor(opts: DeadManSwitchOptions = {}) {
    super();
    this.maxFailures = opts.maxConsecutiveFailures ?? 3;
  }

  isHalted(): boolean {
    return this.halted;
  }

  getHaltEvent(): HaltEvent | null {
    return this.haltEvent;
  }

  getConsecutiveFailures(): number {
    return this.consecutiveFailures;
  }

  recordSuccess(): void {
    this.consecutiveFailures = 0;
  }

  recordFailure(reason: string): void {
    if (this.halted) return;
    this.consecutiveFailures += 1;

    logger.warn(
      { consecutiveFailures: this.consecutiveFailures, maxFailures: this.maxFailures, reason },
      'dead-man-switch: transaction failure recorded',
    );

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
  reset(): void {
    this.consecutiveFailures = 0;
    this.halted = false;
    this.haltEvent = null;
    logger.info('dead-man-switch: reset by operator');
  }
}
