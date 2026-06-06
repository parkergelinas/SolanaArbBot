import { describe, expect, it } from 'vitest';

import { computeAnalytics } from '../src/analytics/metrics.js';
import type { JournalEvent } from '../src/state/journal.js';

describe('computeAnalytics', () => {
  it('aggregates hit rate and rejection reasons', () => {
    const events: JournalEvent[] = [
      { type: 'scan', timestampMs: 1 },
      { type: 'scan', timestampMs: 2 },
      {
        type: 'decision',
        timestampMs: 3,
        expectedProfitUsd: 1.2,
        pairLabel: 'SOL/USDC',
      },
      {
        type: 'rejection',
        timestampMs: 4,
        rejectionReason: 'stale_quote',
      },
      {
        type: 'execution',
        timestampMs: 5,
        expectedProfitUsd: 1.2,
        realizedProfitUsd: 0.9,
        pairLabel: 'SOL/USDC',
      },
    ];

    const snap = computeAnalytics(events);
    expect(snap.totalScans).toBe(2);
    expect(snap.totalExecutions).toBe(1);
    expect(snap.avgQuoteExecutionDriftUsd).toBeCloseTo(-0.3, 2);
    expect(snap.rejectionReasons['stale_quote']).toBe(1);
  });
});
