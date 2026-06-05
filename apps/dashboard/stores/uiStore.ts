import { create } from 'zustand';

import type { CandleInterval } from '@/lib/stream/types';

interface UiState {
  selectedMint: string | null;
  candleInterval: CandleInterval;
  swapTapePaused: boolean;

  setSelectedMint: (mint: string | null) => void;
  setCandleInterval: (interval: CandleInterval) => void;
  setSwapTapePaused: (paused: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  selectedMint: 'So11111111111111111111111111111111111111112',
  candleInterval: '1s',
  swapTapePaused: false,

  setSelectedMint: (mint) => set({ selectedMint: mint }),
  setCandleInterval: (interval) => set({ candleInterval: interval }),
  setSwapTapePaused: (paused) => set({ swapTapePaused: paused }),
}));
