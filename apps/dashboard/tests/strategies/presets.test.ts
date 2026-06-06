import { describe, expect, it } from 'vitest';

import {
  BUILTIN_PRESETS,
  createCustomPresetId,
  enabledStrategyLabels,
  findPreset,
  presetDisplayName,
} from '@/lib/strategies/presets';

describe('strategy presets', () => {
  it('includes built-in balanced preset', () => {
    expect(findPreset('balanced')?.name).toBe('Balanced');
  });

  it('lists enabled engines', () => {
    const preset = BUILTIN_PRESETS.find((p) => p.id === 'whale_hunter')!;
    expect(enabledStrategyLabels(preset.config)).toEqual(['Whale', 'Momentum']);
  });

  it('creates unique custom ids', () => {
    const a = createCustomPresetId('My Strat');
    const b = createCustomPresetId('My Strat');
    expect(a).toMatch(/^custom-my-strat-/);
    expect(a).not.toBe(b);
  });

  it('formats preset display names', () => {
    expect(presetDisplayName('custom-draft')).toBe('Custom draft');
    expect(presetDisplayName('backtest-combined')).toBe('Backtest · combined');
    expect(presetDisplayName('balanced')).toBe('Balanced');
  });
});
