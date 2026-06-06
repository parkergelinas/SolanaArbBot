import { beforeEach, describe, expect, it } from 'vitest';

import type { WSMessage } from '@/lib/stream/types';
import { useMarketStore } from '@/stores/marketStore';
import { makeSwap, swapMessage } from '../helpers/fixtures';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

describe('marketStore', () => {
  beforeEach(() => {
    useMarketStore.getState().clear();
  });

  it('applies swap messages and updates token rows', () => {
    const swap = makeSwap();
    useMarketStore.getState().applyMessages([swapMessage(swap)], { seq: 1, ts_ms: Date.now() });
    const state = useMarketStore.getState();
    expect(state.swaps).toHaveLength(1);
    expect(state.tokens[USDC]).toBeDefined();
    expect(state.tokens[USDC].volume).toBeGreaterThan(0);
  });

  it('caps swap buffer at MAX_SWAPS (500)', () => {
    const messages: WSMessage[] = Array.from({ length: 600 }, (_, i) =>
      swapMessage(makeSwap({ signature: `sig_${i}` })),
    );
    useMarketStore.getState().applyMessages(messages, { seq: 1, ts_ms: Date.now() });
    expect(useMarketStore.getState().swaps).toHaveLength(500);
    expect(useMarketStore.getState().swaps[0].signature).toBe('sig_100');
  });

  it('caps signals at MAX_SIGNALS (80)', () => {
    const signals: WSMessage[] = Array.from({ length: 100 }, (_, i) => ({
      type: 'signal' as const,
      payload: {
        v: 1 as const,
        signal_id: `s${i}`,
        mint: SOL,
        kind: 'momentum' as const,
        strength: 0.5,
        confidence: 0.5,
        timestamp_ms: Date.now(),
      },
    }));
    useMarketStore.getState().applyMessages(signals, { seq: 1, ts_ms: Date.now() });
    expect(useMarketStore.getState().signals).toHaveLength(80);
  });

  it('isolates clear from partial mutation', () => {
    useMarketStore.getState().applyMessages([swapMessage(makeSwap())], { seq: 1, ts_ms: 1 });
    useMarketStore.getState().clear();
    const s = useMarketStore.getState();
    expect(s.swaps).toHaveLength(0);
    expect(s.tokens).toEqual({});
    expect(s.lastSeq).toBe(0);
  });

  it('dedupes swaps by signature', () => {
    const swap = makeSwap({ signature: 'dup_sig' });
    useMarketStore.getState().applyMessages(
      [swapMessage(swap), swapMessage(swap), swapMessage(makeSwap({ signature: 'other' }))],
      { seq: 1, ts_ms: Date.now() },
    );
    const sigs = useMarketStore.getState().swaps.map((s) => s.signature);
    expect(sigs).toEqual(['dup_sig', 'other']);
  });

  it('detects cross-DEX arb when spread exceeds threshold', () => {
    const base = { token_in: SOL, token_out: USDC, timestamp_ms: Date.now() };
    useMarketStore.getState().applyMessages(
      [
        swapMessage(makeSwap({ ...base, dex: 'raydium', amount_in: '1000000000', amount_out: '140000000' })),
        swapMessage(makeSwap({ ...base, dex: 'orca', amount_in: '1000000000', amount_out: '150000000' })),
      ],
      { seq: 2, ts_ms: Date.now() },
    );
    expect(useMarketStore.getState().arbOpportunities.length).toBeGreaterThan(0);
  });
});
