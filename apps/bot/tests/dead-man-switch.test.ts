/**
 * Tests for DeadManSwitch.
 * Verifies halt after 3 consecutive failures, reset behaviour, and event emission.
 */

import { describe, it, expect, vi } from 'vitest';
import { DeadManSwitch } from '../src/execution/dead-man-switch.js';

describe('DeadManSwitch', () => {
  it('is not halted initially', () => {
    const sw = new DeadManSwitch();
    expect(sw.isHalted()).toBe(false);
    expect(sw.getConsecutiveFailures()).toBe(0);
  });

  it('halts after the configured number of consecutive failures (default 3)', () => {
    const sw = new DeadManSwitch();
    sw.recordFailure('err1');
    expect(sw.isHalted()).toBe(false);
    sw.recordFailure('err2');
    expect(sw.isHalted()).toBe(false);
    sw.recordFailure('err3');
    expect(sw.isHalted()).toBe(true);
  });

  it('respects a custom maxConsecutiveFailures', () => {
    const sw = new DeadManSwitch({ maxConsecutiveFailures: 1 });
    sw.recordFailure('instant_halt');
    expect(sw.isHalted()).toBe(true);
  });

  it('emits a halt event with metadata', () => {
    const sw = new DeadManSwitch();
    const handler = vi.fn();
    sw.on('halt', handler);

    sw.recordFailure('a');
    sw.recordFailure('b');
    sw.recordFailure('c');

    expect(handler).toHaveBeenCalledOnce();
    const evt = handler.mock.calls[0][0] as { reason: string; consecutiveFailures: number };
    expect(evt.consecutiveFailures).toBe(3);
    expect(evt.reason).toContain('c');
  });

  it('resets consecutive failure counter on success', () => {
    const sw = new DeadManSwitch();
    sw.recordFailure('e1');
    sw.recordFailure('e2');
    sw.recordSuccess();
    expect(sw.getConsecutiveFailures()).toBe(0);
    expect(sw.isHalted()).toBe(false);
  });

  it('does not increase failure count after halt', () => {
    const sw = new DeadManSwitch({ maxConsecutiveFailures: 2 });
    sw.recordFailure('a');
    sw.recordFailure('b'); // halted now
    const prevCount = sw.getConsecutiveFailures();
    sw.recordFailure('c'); // should be ignored
    expect(sw.getConsecutiveFailures()).toBe(prevCount);
  });

  it('does not emit halt more than once without reset', () => {
    const sw = new DeadManSwitch({ maxConsecutiveFailures: 2 });
    const handler = vi.fn();
    sw.on('halt', handler);
    sw.recordFailure('a');
    sw.recordFailure('b'); // halts
    sw.recordFailure('c'); // no-op
    expect(handler).toHaveBeenCalledOnce();
  });

  it('can be reset and re-halted', () => {
    const sw = new DeadManSwitch();
    sw.recordFailure('a');
    sw.recordFailure('b');
    sw.recordFailure('c');
    expect(sw.isHalted()).toBe(true);

    sw.reset();
    expect(sw.isHalted()).toBe(false);
    expect(sw.getConsecutiveFailures()).toBe(0);
    expect(sw.getHaltEvent()).toBeNull();

    // Can halt again after reset
    sw.recordFailure('x');
    sw.recordFailure('y');
    sw.recordFailure('z');
    expect(sw.isHalted()).toBe(true);
  });

  it('getHaltEvent() returns null before halt and populated after halt', () => {
    const sw = new DeadManSwitch();
    expect(sw.getHaltEvent()).toBeNull();
    sw.recordFailure('a');
    sw.recordFailure('b');
    sw.recordFailure('c');
    const evt = sw.getHaltEvent();
    expect(evt).not.toBeNull();
    expect(evt!.haltedAtMs).toBeLessThanOrEqual(Date.now());
  });
});

// ─── Integration: HardenedExecutor halts after 3 failed executions ───────────

describe('HardenedExecutor dead-man switch integration', () => {
  it('rejects all executions once halted', async () => {
    const sw = new DeadManSwitch({ maxConsecutiveFailures: 3 });

    // Simulate the executor checking the switch
    const executeGated = (decision: { id: string }): { success: boolean; error?: string } => {
      if (sw.isHalted()) return { success: false, error: 'dead_man_switch_halted' };
      // Simulate failure
      sw.recordFailure(`fail_${decision.id}`);
      return { success: false, error: 'rpc_error' };
    };

    const r1 = executeGated({ id: '1' });
    const r2 = executeGated({ id: '2' });
    const r3 = executeGated({ id: '3' }); // triggers halt
    const r4 = executeGated({ id: '4' }); // gated by halt

    expect(r1.error).toBe('rpc_error');
    expect(r2.error).toBe('rpc_error');
    expect(r3.error).toBe('rpc_error');
    expect(r4.error).toBe('dead_man_switch_halted');
    expect(sw.isHalted()).toBe(true);
  });
});
