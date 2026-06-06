import { describe, expect, it } from 'vitest';

import {
  formatPumpDetail,
  isPumpSignal,
  pumpSignalKind,
} from '@/lib/terminal/pumpSignals';
import type { Signal } from '@/lib/stream/types';

const base: Signal = {
  v: 1,
  signal_id: 'x',
  mint: 'mint',
  kind: 'momentum',
  strength: 1,
  confidence: 0.9,
  timestamp_ms: 1,
};

describe('pumpSignals', () => {
  it('detects pump signals by detail prefix', () => {
    expect(isPumpSignal({ ...base, detail: 'pump_launch:liq_sol=1' })).toBe(true);
    expect(isPumpSignal({ ...base, detail: 'flow_imbalance=0.5' })).toBe(false);
  });

  it('classifies launch vs curve', () => {
    expect(pumpSignalKind({ ...base, detail: 'pump_launch:a=1' })).toBe('launch');
    expect(pumpSignalKind({ ...base, detail: 'pump_curve:symbol=ABC' })).toBe('curve');
  });

  it('formats pump detail for UI', () => {
    expect(
      formatPumpDetail('pump_launch:creator=x;liq_sol=2.5;sig=abc'),
    ).toContain('2.50 SOL');
    expect(
      formatPumpDetail('pump_curve:symbol=BONK;grad_pct=45.2;buys_m5=12'),
    ).toContain('BONK');
  });
});
