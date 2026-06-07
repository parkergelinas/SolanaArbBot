import { EventEmitter } from 'events';
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
export declare class DeadManSwitch extends EventEmitter {
    private readonly maxFailures;
    private consecutiveFailures;
    private halted;
    private haltEvent;
    constructor(opts?: DeadManSwitchOptions);
    isHalted(): boolean;
    getHaltEvent(): HaltEvent | null;
    getConsecutiveFailures(): number;
    recordSuccess(): void;
    recordFailure(reason: string): void;
    /** Re-enable trading after operator review. */
    reset(): void;
}
