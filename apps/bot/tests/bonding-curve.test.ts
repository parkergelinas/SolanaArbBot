import { describe, expect, it } from 'vitest';
import { parseLaunchNotification } from '../src/pump/launch-monitor.js';

describe('pump launch monitor', () => {
  it('parses Create instruction from logsNotification', () => {
    const payload = JSON.stringify({
      jsonrpc: '2.0',
      method: 'logsNotification',
      params: {
        result: {
          value: {
            signature: 'abc123',
            logs: ['Program log: Instruction: Create', 'mint: DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263'],
            context: { slot: 12345 },
          },
        },
      },
    });

    const event = parseLaunchNotification(payload);
    expect(event).not.toBeNull();
    expect(event!.signature).toBe('abc123');
    expect(event!.slot).toBe(12345);
    expect(event!.mint).toContain('DezX');
  });

  it('ignores non-Create logs', () => {
    const payload = JSON.stringify({
      params: {
        result: {
          value: {
            signature: 'x',
            logs: ['Instruction: Buy'],
            context: { slot: 1 },
          },
        },
      },
    });
    expect(parseLaunchNotification(payload)).toBeNull();
  });
});
