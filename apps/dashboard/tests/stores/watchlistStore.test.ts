import { beforeEach, describe, expect, it } from 'vitest';

import { SOL_MINT } from '@/lib/terminal/tokens';
import { useWatchlistStore } from '@/stores/watchlistStore';

const CUSTOM_MINT = '7GCihgDB8fe6KNjn2MYtkz9cRjQ3K1qE8Y3o6SolCustom';

describe('watchlistStore', () => {
  beforeEach(() => {
    useWatchlistStore.setState({
      entries: useWatchlistStore.getState().entries.filter((e) => !e.custom),
      hydrated: true,
      addOpen: false,
    });
  });

  it('includes default SOL in entries', () => {
    const entries = useWatchlistStore.getState().entries;
    expect(entries.some((e) => e.mint === SOL_MINT && !e.custom)).toBe(true);
  });

  it('adds custom token', () => {
    const ok = useWatchlistStore.getState().addToken({
      mint: CUSTOM_MINT,
      symbol: 'BONK',
      name: 'Bonk',
    });
    expect(ok).toBe(true);
    expect(useWatchlistStore.getState().entries.some((e) => e.mint === CUSTOM_MINT)).toBe(true);
  });

  it('rejects duplicate mint', () => {
    useWatchlistStore.getState().addToken({
      mint: CUSTOM_MINT,
      symbol: 'BONK',
      name: 'Bonk',
    });
    const ok = useWatchlistStore.getState().addToken({
      mint: CUSTOM_MINT,
      symbol: 'BONK',
      name: 'Bonk',
    });
    expect(ok).toBe(false);
  });

  it('removes custom token only', () => {
    useWatchlistStore.getState().addToken({
      mint: CUSTOM_MINT,
      symbol: 'BONK',
      name: 'Bonk',
    });
    expect(useWatchlistStore.getState().removeToken(CUSTOM_MINT)).toBe(true);
    expect(useWatchlistStore.getState().entries.some((e) => e.mint === CUSTOM_MINT)).toBe(false);
  });

  it('cannot remove default SOL', () => {
    expect(useWatchlistStore.getState().removeToken(SOL_MINT)).toBe(false);
  });
});
